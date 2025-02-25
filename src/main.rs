use simple_logger::SimpleLogger;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use log::{info, warn, error};

impl Default for Player {
    fn default() -> Self {
        Player {
            id: SpecialPlayers::SelfPlayer as u8,
            username: "".to_string(),
            verification_key: [0; 64],
            unused: 0x00,
            position_x: 0,
            position_y: 0,
            position_z: 0,
            yaw: 0,
            pitch: 0,
            operator: false,
            outgoing_data: Vec::new(),
        }
    }
}

struct Player {
    // Struct `Player` is never constructed `#[warn(fuck_you)]` on by default
    pub id: u8,
    pub username: String,
    pub verification_key: [u8; 64],
    pub unused: u8,
    pub position_x: i16,
    pub position_y: i16,
    pub position_z: i16,
    pub yaw: u8,
    pub pitch: u8,
    pub operator: bool,
    pub outgoing_data: Vec<u8>,
}

enum SpecialPlayers {
    SelfPlayer = 0xFF,
}

#[derive(Copy, Clone, PartialEq)]
enum PlayerStatus {
    Disconnected,
    ConnectedSelf,
    Connected,
}

struct World {
    pub size_x: i16,
    pub size_y: i16,
    pub size_z: i16,
    pub data: Vec<u8>,
}

fn build_world(size_x: i16, size_y: i16, size_z: i16) -> Vec<u8> {
    let mut world_dat: Vec<u8> = Vec::new();

    for y in 0..size_y {
        for _z in 0..size_z {
            for _x in 0..size_x {
                if y < 15 {
                    world_dat.push(3); // Dirt
                } else if y == 15 {
                    world_dat.push(2); // Grass
                } else {
                    world_dat.push(0x00); // Air
                }
            }
        }
    }

    return world_dat;
}

const SIZE_X: i16 = 64;
const SIZE_Y: i16 = 32;
const SIZE_Z: i16 = 64;

fn handle_client(
    mut stream: TcpStream,
    client_number: u8,
    players_arc_clone: Arc<Mutex<[Player; 255]>>,
    world_arc_clone: Arc<Mutex<World>>,
) {
    thread::spawn(move || {
        info!("Thread initialized with player ID: {}", client_number);

        let mut player_statuses = [PlayerStatus::Disconnected; 255];
        let mut immediate_join = [false; 255];
        {
            let mut players = players_arc_clone.lock().unwrap();
            for i in 0..players.len() {
                let current_player = &mut players[i];
                match current_player.id {
                    255 => {
                        continue;
                    }
                    _ => {
                        player_statuses[i] = PlayerStatus::Connected;
                        immediate_join[i] = true;
                    }
                }
            }
        }
        player_statuses[client_number as usize] = PlayerStatus::ConnectedSelf;

        loop {
            let mut buffer = [0; 1];
            let _ = stream.read(&mut buffer);

            match buffer[0] {
                0x00 => {
                    let mut payload_buffer = [0; 130]; // Byte + String + String + Byte
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != 7 {
                        // Shit pant
                        let _ = &mut stream.write(&client_disconnect(
                            "Something went wrong (CODE: PACKET_SKIPPED)",
                        ));
                        warn!("Something went wrong, packet 0x00 received but second byte was not 7");
                        break;
                    }

                    let mut username = String::new();

                    for i in 0..64 {
                        username.push(payload_buffer[i + 1] as char);
                    }

                    let mut verif_key = [0; 64];

                    for i in 0..64 {
                        verif_key[i] = payload_buffer[i + 65];
                    }

                    let mut verif_key_formatted = String::new();
                    use std::fmt::Write;
                    for &byte in &verif_key {
                        write!(&mut verif_key_formatted, "{:X}", byte).expect("Piss");
                    }
                    {
                        let mut players = players_arc_clone.lock().unwrap();
                        let current_player = &mut players[client_number as usize];

                        current_player.id = client_number;
                        current_player.username = username.trim().to_string();
                        current_player.verification_key = verif_key;
                        current_player.unused = payload_buffer[129];
                        current_player.position_x = 0;
                        current_player.position_y = 128;
                        current_player.position_z = 0;
                        current_player.yaw = 0;
                        current_player.pitch = 0;
                        current_player.operator = true;

                        bomb_server_details(&mut stream, &current_player, &world_arc_clone);

                        for i in 0..immediate_join.len() {
                            if immediate_join[i] {
                                //println!("Immediately joining {}", i);
                                let _ = &mut stream.write(&spawn_player(
                                    players[i].id,
                                    &players[i].username,
                                    players[i].position_x,
                                    players[i].position_y,
                                    players[i].position_z,
                                    players[i].yaw,
                                    players[i].pitch,
                                ));
                            }
                        }
                    }
                }
                0x05 => {
                    let mut payload_buffer = [0; 8]; // Short (2) + Short (2) + Short (2) + Byte (1) + Byte (1)
                    let _ = stream.read(&mut payload_buffer);

                    let position_x =
                        ((payload_buffer[0] as i16) << (8 as i16)) + payload_buffer[1] as i16;
                    let position_y =
                        ((payload_buffer[2] as i16) << (8 as i16)) + payload_buffer[3] as i16;
                    let position_z =
                        ((payload_buffer[4] as i16) << (8 as i16)) + payload_buffer[5] as i16;

                    let mode = payload_buffer[6];
                    let mut block_type = payload_buffer[7];
                    {
                        let mut world_dat = world_arc_clone.lock().unwrap();

                        // Sanity check (Stop losers from losing)
                        if position_x > world_dat.size_x
                            || position_y > world_dat.size_y
                            || position_z > world_dat.size_z
                        {
                            // Fuck you!
                            let _ = &mut stream.write(&client_disconnect(
                                "Block position was not within world bounds, naughty boy",
                            ));
                            break;
                        }

                        if mode == 0x00 {
                            block_type = 0x00; // Air
                        }

                        let world_offset: u32 = position_x as u32
                            + (position_z as u32 * world_dat.size_x as u32)
                            + (position_y as u32
                                * world_dat.size_x as u32
                                * world_dat.size_z as u32);
                        world_dat.data[world_offset as usize] = block_type;
                    }

                    let mut update_block_bytes: Vec<u8> = Vec::new();
                    update_block_bytes.push(0x06);
                    update_block_bytes.extend_from_slice(&stream_write_short(position_x));
                    update_block_bytes.extend_from_slice(&stream_write_short(position_y));
                    update_block_bytes.extend_from_slice(&stream_write_short(position_z));
                    update_block_bytes.push(block_type);

                    let mut players = players_arc_clone.lock().unwrap();
                    for i in 0..players.len() {
                        if players[i].id != 255 && players[i].id != client_number {
                            players[i]
                                .outgoing_data
                                .extend_from_slice(&update_block_bytes);
                        }
                    }
                }
                0x08 => {
                    let mut payload_buffer = [0; 9]; // SByte + FShort (2B) + FShort + FShort +
                                                     // Byte + Byte
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != SpecialPlayers::SelfPlayer as u8 {
                        let _ = &mut stream.write(&client_disconnect("Evil bit level hacking"));
                        break;
                    }
                    {
                        let mut players = players_arc_clone.lock().unwrap();
                        let current_player = &mut players[client_number as usize];
                        current_player.position_x =
                            ((payload_buffer[1] as i16) << (8 as i16)) + payload_buffer[2] as i16;
                        current_player.position_y =
                            ((payload_buffer[3] as i16) << (8 as i16)) + payload_buffer[4] as i16;
                        current_player.position_z =
                            ((payload_buffer[5] as i16) << (8 as i16)) + payload_buffer[6] as i16;

                        current_player.yaw = payload_buffer[7];
                        current_player.pitch = payload_buffer[8];
                    }
                }
                0x0D => {
                    let mut payload_buffer = [0; 65]; // Byte + String
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != SpecialPlayers::SelfPlayer as u8 {
                        let _ = &mut stream.write(&client_disconnect("Evil bit level hacking"));
                        break;
                    }

                    let mut message = ['a'; 64];
                    for i in 0..64 {
                        message[i] = payload_buffer[i + 1] as char;
                    }

                    let mut players = players_arc_clone.lock().unwrap();
                    for i in 0..players.len() {
                        if players[i].id != 255 && players[i].id != client_number {
                            let sender: u8 = players[client_number as usize].id;
                            players[i]
                                .outgoing_data
                                .extend_from_slice(&send_chat_message(
                                    sender,
                                    String::from_iter(message),
                                ));
                        }
                    }

                    let _ = &mut stream.write(&send_chat_message(
                        SpecialPlayers::SelfPlayer as u8,
                        String::from_iter(message),
                    ));
                    info!("{}", String::from_iter(message));
                }
                _ => warn!("Packet {} not implemented!", buffer[0]),
            }
            let is_kill = &mut stream.write(&ping()); // Ping that MF

            if is_kill.is_err() {
                break;
            }

            sleep(Duration::from_millis(1000 / 1000)); // 1000 TPS  TODO: Delta time
            {
                let mut players = players_arc_clone.lock().unwrap();
                if players[client_number as usize].outgoing_data.len() > 0 {
                    let _ = stream.write(&players[client_number as usize].outgoing_data);
                    players[client_number as usize].outgoing_data.clear();
                }
                for i in 0..players.len() {
                    if players[i].id != 255 {
                        if player_statuses[i] == PlayerStatus::Disconnected {
                            let _ = stream.write(&spawn_player(
                                players[i].id,
                                &players[i].username,
                                players[i].position_x,
                                players[i].position_y,
                                players[i].position_z,
                                players[i].yaw,
                                players[i].pitch,
                            ));
                            player_statuses[i] = PlayerStatus::Connected;
                            let _ = stream.write(&send_chat_message(
                                players[i].id,
                                format!("{} has joined the game!", &players[i].username),
                            ));
                        }
                    } else {
                        if player_statuses[i] == PlayerStatus::Connected {
                            let _ = stream.write(&despawn_player(i.try_into().unwrap()));
                            let _ = stream.write(&send_chat_message(
                                i.try_into().unwrap(),
                                format!("{} has left the game!", &players[i].username),
                            ));
                            player_statuses[i] = PlayerStatus::Disconnected;
                        }
                    }
                    if player_statuses[i] == PlayerStatus::Connected {
                        let _ = stream.write(&set_position_and_orientation(
                            players[i].id,
                            players[i].position_x,
                            players[i].position_y,
                            players[i].position_z,
                            players[i].yaw,
                            players[i].pitch,
                        ));
                    }
                }
            }
        }
        {
            let mut players = players_arc_clone.lock().unwrap();
            let current_player = &mut players[client_number as usize];

            current_player.id = SpecialPlayers::SelfPlayer as u8;
        }
        info!("Client {} disconnected, thread shutting down!", client_number);
    });
}

fn to_mc_string(text: &str) -> [u8; 64] {
    let text_vec: Vec<char> = text.chars().take(64).collect();
    let mut balls = [0; 64];

    for i in 0..64 {
        balls[i] = 0x20;
    }

    for i in 0..text_vec.len() {
        balls[i] = text_vec[i] as u8;
    }

    return balls;
}

fn stream_write_short(data: i16) -> Vec<u8> {
    return [(data >> 0x08) as u8, (data & 0x00FF) as u8].to_vec();
}

fn client_disconnect(text: &str) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x0E);
    ret_val.append(&mut to_mc_string(text).to_vec());
    return ret_val;
}

fn server_identification(is_op: bool) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x00);
    ret_val.push(0x07);

    let server_name = "Erm... what the sigma?";
    ret_val.append(&mut to_mc_string(server_name).to_vec());

    let server_motd = "Pragmatism not idealism";
    ret_val.append(&mut to_mc_string(server_motd).to_vec());

    if is_op {
        ret_val.push(0x64);
    } else {
        ret_val.push(0x00);
    }

    return ret_val;
}

fn ping() -> Vec<u8> {
    return vec![0x01];
}

fn init_level() -> Vec<u8> {
    return vec![0x02];
}

fn finalize_level(world_arc_clone: &Arc<Mutex<World>>) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x04);

    let world_dat = world_arc_clone.lock().unwrap();

    ret_val.append(&mut stream_write_short(world_dat.size_x).to_vec());
    ret_val.append(&mut stream_write_short(world_dat.size_y).to_vec());
    ret_val.append(&mut stream_write_short(world_dat.size_z).to_vec());

    return ret_val;
}

fn spawn_player(
    player_id: u8,
    name: &String,
    pos_x: i16,
    pos_y: i16,
    pos_z: i16,
    yaw: u8,
    pitch: u8,
) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x07);

    ret_val.push(player_id);
    ret_val.append(&mut to_mc_string(name).to_vec());
    ret_val.append(&mut stream_write_short(pos_x << 5).to_vec()); // FShort
    ret_val.append(&mut stream_write_short(pos_y << 5).to_vec());
    ret_val.append(&mut stream_write_short(pos_z << 5).to_vec());
    ret_val.push(yaw);
    ret_val.push(pitch);

    return ret_val;
}

fn despawn_player(player_id: u8) -> Vec<u8> {
    return [0x0C, player_id].to_vec();
}

fn send_chat_message(source_id: u8, message: String) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x0D);

    ret_val.push(source_id);
    ret_val.append(&mut to_mc_string(&message).to_vec());

    return ret_val;
}

fn set_position_and_orientation(
    player_id: u8,
    pos_x: i16,
    pos_y: i16,
    pos_z: i16,
    yaw: u8,
    pitch: u8,
) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x08);

    ret_val.push(player_id);
    ret_val.append(&mut stream_write_short(pos_x).to_vec());
    ret_val.append(&mut stream_write_short(pos_y).to_vec());
    ret_val.append(&mut stream_write_short(pos_z).to_vec());

    ret_val.push(yaw);
    ret_val.push(pitch);

    return ret_val;
}

fn send_level_data(world_arc_clone: &Arc<Mutex<World>>) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    let mut world_dat = world_arc_clone.lock().unwrap().data.clone();

    // Big endian fold lmao
    world_dat.insert(0, ((world_dat.len() & 0xFF) >> 0) as u8);
    world_dat.insert(0, ((world_dat.len() & 0xFF00) >> 8) as u8);
    world_dat.insert(0, ((world_dat.len() & 0xFF0000) >> 16) as u8);
    world_dat.insert(0, ((world_dat.len() & 0xFF000000) >> 24) as u8);

    // TODO: Stream GZIP straight onto the network

    let mut world_dat_compressor = GzEncoder::new(Vec::new(), Compression::fast());
    for i in 0..world_dat.len() {
        let _ = world_dat_compressor.write(&[world_dat[i]]);
    }
    let world_dat_gzipped = world_dat_compressor.finish().unwrap();

    let number_of_chunks = ((world_dat_gzipped.len() as f32) / 1024.0_f32).ceil() as usize;
    let mut current_chunk = 0;

    if number_of_chunks != 1 {
        while current_chunk + 1 != number_of_chunks {
            ret_val.push(0x03);

            ret_val.append(&mut stream_write_short(0x400));

            let mut chunk_data_buffer = [0u8; 1024];
            for i in 0..1024 {
                chunk_data_buffer[i] = world_dat_gzipped[current_chunk * 1024 + i];
            }
            ret_val.append(&mut chunk_data_buffer.to_vec());

            let mut percentage = current_chunk / number_of_chunks * 100;

            if percentage > 100 {
                percentage = 100;
            }

            ret_val.push(percentage.try_into().unwrap());

            current_chunk += 1;
        }
    }

    let remaining_chunk_size = world_dat_gzipped.len() - (current_chunk * 1024);

    if remaining_chunk_size > 0 {
        ret_val.push(0x03);

        ret_val.append(&mut stream_write_short(
            remaining_chunk_size.try_into().unwrap(),
        ));

        let mut remaining_data_buffer = [0u8; 1024];
        for i in 0..remaining_chunk_size {
            remaining_data_buffer[i] = world_dat_gzipped[current_chunk * 1024 + i];
        }

        ret_val.append(&mut remaining_data_buffer.to_vec());
        ret_val.push(100);
    }

    info!(
        "World transmission size: {}KiB",
        ret_val.len() as f32 / 1024.0
    );
    return ret_val;
}

fn save_world(world_arc_clone: Arc<Mutex<World>>) -> std::io::Result<()> {
    let mut to_write: Vec<u8> = Vec::new();
    {
        let mut world_dat = world_arc_clone.lock().unwrap();
        to_write.push((world_dat.size_x >> 8) as u8);
        to_write.push((world_dat.size_x & 0xFF) as u8);
        to_write.push((world_dat.size_y >> 8) as u8);
        to_write.push((world_dat.size_y & 0xFF) as u8);
        to_write.push((world_dat.size_z >> 8) as u8);
        to_write.push((world_dat.size_z & 0xFF) as u8);
        to_write.append(&mut world_dat.data);
    }

    let mut file = File::create("world.wrld")?;
    return file.write_all(&to_write);
}

fn load_world() -> World {
    if fs::metadata("world.wrld").is_ok() {
        let mut world: World = World {
            size_x: 0,
            size_y: 0,
            size_z: 0,
            data: Vec::new(),
        };
        let world_data_raw = fs::read("world.wrld").unwrap();
        if world_data_raw.len() < 6 {
            error!("INVALID WORLD!");
            std::process::exit(1);
        }
        world.size_x = ((world_data_raw[0] as i16) << 8) + (world_data_raw[1] as i16);
        world.size_y = ((world_data_raw[2] as i16) << 8) + (world_data_raw[3] as i16);
        world.size_z = ((world_data_raw[4] as i16) << 8) + (world_data_raw[5] as i16);

        if world_data_raw.len()
            != (world.size_x as i32 * world.size_y as i32 * world.size_z as i32 + 6 as i32) as usize
        {
            error!(
                "Expected more bytes in world contents: {} (expected) != {} (actual)",
                world.size_x * world.size_y * world.size_z + 6,
                world_data_raw.len()
            );
            std::process::exit(1);
        }

        world.data = world_data_raw[6..world_data_raw.len()].to_vec();

        return world;
    } else {
        return World {
            size_x: SIZE_X,
            size_y: SIZE_Y,
            size_z: SIZE_Z,
            data: build_world(SIZE_X, SIZE_Y, SIZE_Z),
        };
    }
}

fn bomb_server_details(
    stream: &mut TcpStream,
    current_player: &Player,
    world_arc_clone: &Arc<Mutex<World>>,
) {
    let mut compound_data: Vec<u8> = vec![];
    info!("Server IDENT");
    compound_data.append(&mut server_identification(current_player.operator));

    info!("Intialize level");
    compound_data.append(&mut init_level());

    info!("Send level data");
    compound_data.append(&mut send_level_data(&world_arc_clone)); // Approaching Nirvana - Maw of the beast

    info!("Finalize level");
    compound_data.append(&mut finalize_level(&world_arc_clone));

    info!("Spawning player");
    compound_data.append(&mut spawn_player(
        SpecialPlayers::SelfPlayer as u8,
        &current_player.username,
        32,
        17,
        32,
        0,
        0,
    ));

    let _ = stream.write(&compound_data);
}

fn main() -> std::io::Result<()> {
    SimpleLogger::new().with_threads(true).init().unwrap();

    let players: [Player; 255] = core::array::from_fn(|_| Player::default());
    let players_arc = Arc::new(Mutex::new(players));

    let world_instance: World = load_world();
    let world_arc = Arc::new(Mutex::new(world_instance));

    let addr = SocketAddr::from(([0, 0, 0, 0], 25565));

    let listener = TcpListener::bind(addr)?;

    let mut thread_number: u8 = 0;

    let world_arc_clone_main_thread = Arc::clone(&world_arc);
    ctrlc::set_handler(move || {
        let _ = save_world(world_arc_clone_main_thread.clone()); // Fortnite save the world
        std::process::exit(0);
    })
    .expect("Error handling control C, save on exit will not work");

    for stream in listener.incoming() {
        let players_arc_clone = Arc::clone(&players_arc);
        let world_arc_clone = Arc::clone(&world_arc);
        handle_client(stream?, thread_number, players_arc_clone, world_arc_clone);
        if thread_number < 255 {
            thread_number += 1;
        } else {
            thread_number = 0;
        }
    }
    Ok(())
}
