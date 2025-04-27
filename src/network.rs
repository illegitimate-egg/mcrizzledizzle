use log::{info, warn};
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use crate::command::handle_command;
use crate::config::ServerConfig;
use crate::extensions::{Event, EventType, Extensions};
use crate::player::{Player, PlayerStatus, SpecialPlayers};
use crate::utils::*;
use crate::world::World;

pub fn handle_client(
    config: ServerConfig,
    mut stream: TcpStream,
    client_number: u8,
    players_arc_clone: Arc<Mutex<[Player; 255]>>,
    world_arc_clone: Arc<Mutex<World>>,
    extensions: Arc<Extensions>,
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
            if stream.read(&mut buffer).unwrap() == 0 {
                break;
            }

            match buffer[0] {
                0x00 => {
                    let mut payload_buffer = [0; 130]; // Byte + String + String + Byte
                    let _ = stream.read(&mut payload_buffer);

                    if payload_buffer[0] != 7 {
                        // Shit pant
                        let _ = &mut stream.write(&client_disconnect(
                            "Something went wrong (Type 0x00 expected ver 7, something else was received, contact server admin)",
                        ));
                        warn!("Something went wrong, packet 0x00 received but second byte was not 0x07: received {:#04X} instead.", payload_buffer[0]);
                        break;
                    }

                    let mut username = String::new();

                    for i in 0..64 {
                        username.push(payload_buffer[i + 1] as char);
                    }

                    let mut verif_key = [0; 64];
                    verif_key.copy_from_slice(&payload_buffer[65..(64 + 65)]);

                    let mut verif_key_formatted = String::new();
                    use std::fmt::Write;
                    for &byte in &verif_key {
                        write!(&mut verif_key_formatted, "{byte:X}").expect("Piss");
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

                        let _ = bomb_server_details(
                            config.clone(),
                            &mut stream,
                            current_player,
                            &world_arc_clone,
                        );

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

                    let mut is_cancelled = false;
                    let mut previous_block: u8 = 0;

                    let position_x =
                        ((payload_buffer[0] as i16) << 8_i16) + payload_buffer[1] as i16;
                    let position_y =
                        ((payload_buffer[2] as i16) << 8_i16) + payload_buffer[3] as i16;
                    let position_z =
                        ((payload_buffer[4] as i16) << 8_i16) + payload_buffer[5] as i16;

                    let mode = payload_buffer[6];
                    let mut block_type = payload_buffer[7];
                    {
                        if mode == 0x00 {
                            // EVENT: BLOCK BREAK
                            let mut event = Event::new();
                            event.player = client_number;
                            event.position.x = position_x;
                            event.position.y = position_y;
                            event.position.z = position_z;
                            event.selected_block = block_type;
                            event = extensions.run_event(EventType::BlockBreak, event);

                            is_cancelled = event.is_cancelled;

                            block_type = 0x00; // Air
                        }

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

                        let world_offset: u32 = position_x as u32
                            + (position_z as u32 * world_dat.size_x as u32)
                            + (position_y as u32
                                * world_dat.size_x as u32
                                * world_dat.size_z as u32);
                        if !is_cancelled {
                            world_dat.data[world_offset as usize] = block_type;
                        } else {
                            previous_block = world_dat.data[world_offset as usize];
                        }
                    }

                    if !is_cancelled {
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
                    } else {
                        let mut update_block_bytes: Vec<u8> = Vec::new();
                        update_block_bytes.push(0x06);
                        update_block_bytes.extend_from_slice(&stream_write_short(position_x));
                        update_block_bytes.extend_from_slice(&stream_write_short(position_y));
                        update_block_bytes.extend_from_slice(&stream_write_short(position_z));
                        update_block_bytes.push(previous_block);

                        let _ = stream.write(&update_block_bytes);
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
                            ((payload_buffer[1] as i16) << 8_i16) + payload_buffer[2] as i16;
                        current_player.position_y =
                            ((payload_buffer[3] as i16) << 8_i16) + payload_buffer[4] as i16;
                        current_player.position_z =
                            ((payload_buffer[5] as i16) << 8_i16) + payload_buffer[6] as i16;

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

                    let mut message = [' '; 64];
                    for i in 0..64 {
                        message[i] = payload_buffer[i + 1] as char;
                    }

                    let message_string = String::from_iter(message);

                    if message[0] == '/' {
                        // Uh oh, command time
                        info!("{}", message_string);
                        let remaning_command = String::from_iter(&message[1..message.len()]);
                        let _ = handle_command(
                            &mut stream,
                            client_number,
                            &players_arc_clone,
                            &extensions,
                            &remaning_command,
                        );
                    } else {
                        let mut players = players_arc_clone.lock().unwrap();
                        let sender: u8 = players[client_number as usize].id;
                        let sender_name: String = players[client_number as usize].username.clone();
                        for i in 0..players.len() {
                            if players[i].id != 255 && players[i].id != client_number {
                                players[i]
                                    .outgoing_data
                                    .extend_from_slice(&send_chat_message(
                                        sender,
                                        sender_name.clone(),
                                        message_string.clone(),
                                    ));
                            }
                        }

                        let _ = &mut stream.write(&send_chat_message(
                            SpecialPlayers::SelfPlayer as u8,
                            sender_name.clone(),
                            message_string.clone(),
                        ));
                        info!("[{}]: {}", sender_name, message_string);
                    }
                }
                _ => warn!("Packet {} not implemented!", buffer[0]),
            }
            let is_kill = &mut stream.write(&ping()); // Ping that MF

            if is_kill.is_err() {
                break;
            }

            sleep(Duration::from_millis(50)); // 1000 TPS  TODO: Delta time
            {
                let mut players = players_arc_clone.lock().unwrap();
                if !players[client_number as usize].outgoing_data.is_empty() {
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
                                "".to_string(),
                                format!("{} has joined the game!", &players[i].username),
                            ));
                        }
                    } else if player_statuses[i] == PlayerStatus::Connected {
                        let _ = stream.write(&despawn_player(i.try_into().unwrap()));
                        let _ = stream.write(&send_chat_message(
                            i.try_into().unwrap(),
                            "".to_string(),
                            format!("{} has left the game!", &players[i].username),
                        ));
                        player_statuses[i] = PlayerStatus::Disconnected;
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
        let mut event = Event::new();
        event.player = client_number;
        extensions.run_event(EventType::PlayerLeave, event);
        info!(
            "Client {} disconnected, thread shutting down!",
            client_number
        );
    });
}
