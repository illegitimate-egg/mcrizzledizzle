use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use flate2::Compression;
use flate2::write::GzEncoder;
use rand::prelude::*;
use colored::Colorize;
#[macro_use]
extern crate lazy_static;

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
                operator: false
        }
    }
}

struct Player { // Struct `Player` is never constructed `#[warn(fuck_you)]` on by default
    pub id: u8,
    pub username: String,
    pub verification_key: [u8; 64],
    pub unused: u8,
    pub position_x: i16,
    pub position_y: i16,
    pub position_z: i16,
    pub yaw: u8,
    pub pitch: u8,
    pub operator: bool
}

enum SpecialPlayers {
    SelfPlayer = 0xFF
}

#[derive(Copy, Clone, PartialEq)]
enum PlayerStatus {
    Disconnected,
    ConnectedSelf,
    Connected
}

struct World {
    pub size_x: i16,
    pub size_y: i16,
    pub size_z: i16,
    pub data: Vec<u8>
}

fn build_world(size_x: i16, size_y: i16, size_z: i16) -> Vec<u8> {
    let mut rng = rand::thread_rng();

    let mut world_dat: Vec<u8> = Vec::new();

    for y in 0..size_y {
        for _z in 0..size_z {
            for _x in 0..size_x {
                if y == 0 {
                    world_dat.push(rng.gen()); // Bookshelf
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

lazy_static!{
    static ref WORLD: World = World {
        size_x: SIZE_X, 
        size_y: SIZE_Y, 
        size_z: SIZE_Z, 
        data: build_world(SIZE_X, SIZE_Y, SIZE_Z)
    };
}

fn handle_client(mut stream: TcpStream, client_number: u8, players_arc_clone: Arc<Mutex<[Player; 255]>>) {
    thread::spawn(move || {
        let mut player_statuses = [PlayerStatus::Disconnected; 255];
        let mut immediate_join = [false; 255];
        {
            let mut players = players_arc_clone.lock().unwrap();
            for i in 0..players.len() {
                let current_player = &mut players[i];
                match current_player.id {
                    255=> {
                        continue;
                    }
                    _=> {
                        player_statuses[i] = PlayerStatus::Connected;
                        immediate_join[i] = true;
                        //println!("Player {} is immediate join!", i);
                    }
                }
            }
        }
        player_statuses[client_number as usize] = PlayerStatus::ConnectedSelf;

        loop {
            let mut buffer = [0; 1];
            let _ = stream.read(&mut buffer);

            match buffer[0] {
                0x00=> {
                    let mut payload_buffer = [0; 130]; // Byte + String + String + Byte
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != 7 {
                        // Shit pant
                        let _ = client_disconnect(&mut stream, "Something went wrong (CODE: PACKET_SKIPPED)");
                        println!("THIS CLIENT IS FUCKED!");
                        break;
                    }

                    //println!("Client Prot Ver: {}", payload_buffer[0]);
                    let mut username = String::new();

                    for i in 0..64 {
                        username.push(payload_buffer[i+1] as char);
                    }
                    //println!("Username: {}", username);

                    let mut verif_key = [0; 64];

                    for i in 0..64 {
                        verif_key[i] = payload_buffer[i+65];
                    }

                    let mut verif_key_formatted = String::new();
                    use std::fmt::Write;
                    for &byte in &verif_key {
                        write!(&mut verif_key_formatted, "{:X}", byte).expect("Piss");
                    }

                    //println!("Verification key: 0x{}", verif_key_formatted);

                    //println!("\"Unused\" Byte: {}", payload_buffer[129]);

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
                        current_player.operator = false;

                        bomb_server_details(&mut stream, &current_player);

                        for i in 0..immediate_join.len() {
                            if immediate_join[i] {
                                //println!("Immediately joining {}", i);
                                let _ = spawn_player(
                                    &mut stream,
                                    players[i].id,
                                    &players[i].username,
                                    players[i].position_x,
                                    players[i].position_y,
                                    players[i].position_z,
                                    players[i].yaw,
                                    players[i].pitch
                                );
                            }
                        }
                    }
                },
                0x08=>{
                    let mut payload_buffer = [0; 9]; // SByte + FShort (2B) + FShort + FShort +
                    // Byte + Byte
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != SpecialPlayers::SelfPlayer as u8 {
                        let _ = client_disconnect(&mut stream, "Evil bit level hacking");
                        break;
                    }
                    {
                        let mut players = players_arc_clone.lock().unwrap();
                        let current_player = &mut players[client_number as usize];
                        current_player.position_x = ((payload_buffer[1] as i16) << (8 as i16)) + payload_buffer[2] as i16;
                        current_player.position_y = ((payload_buffer[3] as i16) << (8 as i16)) + payload_buffer[4] as i16;
                        current_player.position_z = ((payload_buffer[5] as i16) << (8 as i16)) + payload_buffer[6] as i16;

                        current_player.yaw = payload_buffer[7];
                        current_player.pitch = payload_buffer[8];
                    }
                },
                0x0D=>{
                    let mut payload_buffer = [0; 65]; // Byte + String
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != SpecialPlayers::SelfPlayer as u8 {
                        let _ = client_disconnect(&mut stream, "Evil bit level hacking");
                        break;
                    }

                    let mut message = ['a'; 64];
                    for i in 0..64 {
                        message[i] = payload_buffer[i+1] as char;
                    }

                    let _ = send_chat_message(&mut stream, SpecialPlayers::SelfPlayer as u8, String::from_iter(message));
                    println!("{}", String::from_iter(message));
                },
                _=>println!("Packet {} not implemented!", buffer[0]),
            }
        let is_kill = ping(&mut stream); // Ping that MF
        
        if is_kill.is_err() {
            break;
        }

        sleep(Duration::from_millis(50)); // 20 TPS  TODO: Delta time
        {
            let players = players_arc_clone.lock().unwrap();
            for i in 0..players.len() {
                if players[i].id != 255 {
                    if player_statuses[i] == PlayerStatus::Disconnected { 
                        let _ = spawn_player(
                            &mut stream,
                            players[i].id,
                            &players[i].username,
                            players[i].position_x,
                            players[i].position_y,
                            players[i].position_z,
                            players[i].yaw,
                            players[i].pitch
                        );
                        player_statuses[i] = PlayerStatus::Connected;
                        let _ = send_chat_message(&mut stream, players[i].id, format!("{} has joined the game!", &players[i].username));
                    }
                } else {
                    if player_statuses[i] == PlayerStatus::Connected {
                        let _ = despawn_player(&mut stream, i.try_into().unwrap());
                        let _ = send_chat_message(&mut stream, i.try_into().unwrap(), format!("{} has left the game!", &players[i].username));
                        player_statuses[i] = PlayerStatus::Disconnected;
                    }
                }
                if player_statuses[i] == PlayerStatus::Connected {
                    let _ = set_position_and_orientation(
                        &mut stream,
                        players[i].id,
                        players[i].position_x,
                        players[i].position_y,
                        players[i].position_z,
                        players[i].yaw,
                        players[i].pitch
                    );
                }
            }
        }
        }
        { 
            let mut players = players_arc_clone.lock().unwrap();
            let current_player = &mut players[client_number as usize];

            current_player.id = SpecialPlayers::SelfPlayer as u8;
        }
        println!("Thread {} is kill!", client_number); 
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

fn stream_write_array(data: &[u8], stream: &mut TcpStream) -> std::io::Result<()> {
    for i in 0..data.len() {
        stream.write(&[data[i]])?;
    }
    Ok(())
}

fn stream_write_short(data: i16, stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write(&[(data >> 0x08) as u8])?;
    stream.write(&[(data & 0x00FF) as u8])?;

    Ok(())
}

fn client_disconnect(stream: &mut TcpStream, text: &str) -> std::io::Result<()> {
    stream.write(&[0x0E])?; // Disconnect
    stream_write_array(&to_mc_string(text), stream)?;

    Ok(())
}

fn server_identification(stream: &mut TcpStream, is_op: bool) -> std::io::Result<()> {
    stream.write(&[0x00])?;
    stream.write(&[0x07])?;
    
    let server_name = "Erm... what the sigma?";
    stream_write_array(&to_mc_string(server_name), stream)?;

    let server_motd = "Pragmatism not idealism";
    stream_write_array(&to_mc_string(server_motd), stream)?;

    if is_op {
        stream.write(&[0x64])?;
    } else {
        stream.write(&[0x00])?;
    }

    Ok(())
}

fn ping (stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write(&[0x01])?;

    Ok(())
}

fn init_level(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write(&[0x02])?;

    Ok(())
}

fn finalize_level(stream: &mut TcpStream, size_x: i16, size_y: i16, size_z: i16) -> std::io::Result<()> {
    stream.write(&[0x04])?;

    stream_write_short(size_x, stream)?;
    stream_write_short(size_y, stream)?;
    stream_write_short(size_z, stream)?;

    Ok(())
}

fn spawn_player(stream: &mut TcpStream, player_id: u8, name: &String, pos_x: i16, pos_y: i16, pos_z: i16, yaw: u8, pitch: u8) -> std::io::Result<()> {
    stream.write(&[0x07])?;

    stream.write(&[player_id])?; 
    stream_write_array(&to_mc_string(name), stream)?;
    stream_write_short(pos_x, stream)?;
    stream_write_short(pos_y, stream)?;
    stream_write_short(pos_z, stream)?;
    stream.write(&[yaw])?;
    stream.write(&[pitch])?;

    Ok(())
}

fn despawn_player(stream: &mut TcpStream, player_id: u8) -> std::io::Result<()> {
    stream.write(&[0x0c])?;
    stream.write(&[player_id])?;

    Ok(())
}

fn send_chat_message(stream: &mut TcpStream, source_id: u8, message: String) -> std::io::Result<()> {
    stream.write(&[0x0D])?;

    stream.write(&[source_id])?;
    stream_write_array(&to_mc_string(&message), stream)?;

    Ok(())
}

//fn orientation_update(stream: &mut TcpStream, player_id: u8, yaw: u8, pitch: u8) -> std::io::Result<()> {
//    stream.write(&[0x0B])?;
//
//    stream.write(&[player_id])?;
//    stream.write(&[yaw])?;
//    stream.write(&[pitch])?;
//    
//    Ok(())
//}

fn set_position_and_orientation(stream: &mut TcpStream, player_id: u8, pos_x: i16, pos_y: i16, pos_z: i16, yaw: u8, pitch: u8) -> std::io::Result<()> {
    stream.write(&[0x08])?;

    stream.write(&[player_id])?;
 
    stream_write_short(pos_x, stream)?;
    stream_write_short(pos_y, stream)?;
    stream_write_short(pos_z, stream)?;


    stream.write(&[yaw])?;
    stream.write(&[pitch])?;
    Ok(())
}

fn send_level_data(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut world_dat = WORLD.data.clone();

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

    let number_of_chunks = ((world_dat_gzipped.len() as f32)/1024.0_f32).ceil() as usize;
    let mut current_chunk = 0;

    if number_of_chunks != 1 {
        while current_chunk + 1 != number_of_chunks {
            stream.write(&[0x03])?;

            stream_write_short(0x400, stream)?;

            let mut chunk_data_buffer = [0u8; 1024];
            for i in 0..1024 {
                chunk_data_buffer[i] = world_dat_gzipped[current_chunk*1024+i];
            }
            stream_write_array(&chunk_data_buffer, stream)?;

            let mut percentage = current_chunk/number_of_chunks*100;

            if percentage > 100 {
                percentage = 100;
            }

            stream.write(&[percentage.try_into().unwrap()])?;

            current_chunk += 1;
        }
    }

    let remaining_chunk_size = world_dat_gzipped.len() - (current_chunk * 1024);

    if remaining_chunk_size > 0 {
        stream.write(&[0x03])?;
        
        stream_write_short(remaining_chunk_size.try_into().unwrap(), stream)?;

        let mut remaining_data_buffer = [0u8; 1024];
        for i in 0..remaining_chunk_size {
            remaining_data_buffer[i] = world_dat_gzipped[current_chunk*1024+i];
        }

        stream_write_array(&remaining_data_buffer, stream)?;
        stream.write(&[100])?;
    }

    Ok(())
}

fn bomb_server_details(stream: &mut TcpStream, current_player: &Player) {
    //println!("Server IDENT");
    let _ = server_identification(stream, current_player.operator);

    //println!("Intialize level");
    let _ = init_level(stream);

    //println!("Send level data");
    let _ = send_level_data(stream); // Approaching Nirvana - Maw of the beast

    //println!("Finalize level");
    let _ = finalize_level(stream, WORLD.size_x, WORLD.size_y, WORLD.size_z);

    //println!("Spawning player");
    let _ = spawn_player(stream, SpecialPlayers::SelfPlayer as u8, &current_player.username, 64, 2, 64, 0, 0);
}

fn _create_player_info_window(client_number: u8, players_arc_clone: Arc<Mutex<[Player; 255]>>) {
    thread::spawn(move || {
        let mut thread_alive = false;
        loop {
            {
                let mut players = players_arc_clone.lock().unwrap();
                let current_player = &mut players[client_number as usize];

                if current_player.id == 255 && thread_alive == true {
                    println!("[{}{}] {} has disconnected!", "THREAD: ".cyan(), client_number, current_player.username);
                    break;
                } else {
                    thread_alive = true;
                }

                println!("[{}{}] id:    {}", "THREAD: ".cyan(), client_number, current_player.id);
                println!("[{}{}] pos_x: {}", "THREAD: ".cyan(), client_number, current_player.position_x >> 5);
                println!("[{}{}] pos_y: {}", "THREAD: ".cyan(), client_number, current_player.position_y >> 5);
                println!("[{}{}] pos_z: {}", "THREAD: ".cyan(), client_number, current_player.position_z >> 5);
            }
            sleep(Duration::from_millis(200));
        }
    });
}

fn main() -> std::io::Result<()> {
    let players: [Player; 255] = core::array::from_fn(|_| Player::default());
    let players_arc = Arc::new(Mutex::new(players));

    let addr = SocketAddr::from(([0, 0, 0, 0], 25565));

    let listener = TcpListener::bind(addr)?;
    
    let mut thread_number : u8 = 0;

    for stream in listener.incoming() {
        let players_arc_clone = Arc::clone(&players_arc);
        //let _players_arc_clone_debug = Arc::clone(&players_arc);
        handle_client(stream?, thread_number, players_arc_clone);
        //_create_player_info_window(thread_number, _players_arc_clone_debug);
        if thread_number < 255 {
            thread_number += 1;
        } else {
            thread_number = 0;
        }
    }
    Ok(())
}
