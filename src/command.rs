use log::error;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::error::AppError;
use crate::extensions::Extensions;
use crate::player::{Player, SpecialPlayers};
use crate::utils::*;

pub fn handle_command(
    stream: &mut TcpStream,
    client_number: u8,
    players_arc_clone: &Arc<Mutex<[Player; 255]>>,
    extensions: &Arc<Extensions>,
    command_string: &String,
) -> Result<(), AppError> {
    let vectorized_command = command_string.trim().split(" ").collect::<Vec<&str>>();
    match vectorized_command[0] {
        "kick" => {
            if (vectorized_command.len()) == 1 {
                let _ = &mut stream.write(&send_chat_message(
                    SpecialPlayers::SelfPlayer as u8,
                    "".to_string(),
                    "&cUsage: kick [player]".to_string(),
                ));
            } else {
                let mut players = players_arc_clone
                    .lock()
                    .map_err(|e| AppError::MutexPoisoned(e.to_string()))?;
                for i in 0..players.len() {
                    if players[i].id != 255 {
                        if players[i].username == vectorized_command[1] {
                            let _ = &mut players[i]
                                .outgoing_data
                                .extend_from_slice(&client_disconnect("KICKED!"));
                            players[i].id = 255;
                            break;
                        }
                    }
                }
            }
        }
        "tp" => {
            if (vectorized_command.len()) == 1 {
                let _ = &mut stream.write(&send_chat_message(
                    SpecialPlayers::SelfPlayer as u8,
                    "".to_string(),
                    "&cUsage: tp [player]".to_string(),
                ));
            } else {
                let players = players_arc_clone
                    .lock()
                    .map_err(|e| AppError::MutexPoisoned(e.to_string()))?;
                for i in 0..players.len() {
                    if players[i].id != 255 {
                        if players[i].username == vectorized_command[1] {
                            let _ = &mut stream.write(&set_position_and_orientation(
                                SpecialPlayers::SelfPlayer as u8,
                                players[i].position_x,
                                players[i].position_y,
                                players[i].position_z,
                                players[i].yaw,
                                players[i].pitch,
                            ));
                            break;
                        }
                    }
                }
            }
        }
        _ => {
            let mut vectorized_command_object: Vec<String> = Vec::new();
            for arg in &vectorized_command {
                vectorized_command_object.push(arg.to_string());
            }

            let extensions_clone = Arc::clone(extensions);
            let players_clone = Arc::clone(players_arc_clone);
            let command_key = vectorized_command[0].to_string();
            let client_number_copy = client_number;

            // Async thread commands
            std::thread::spawn(move || {
                let result = extensions_clone.run_command(
                    command_key,
                    client_number_copy,
                    vectorized_command_object,
                );

                match result {
                    Ok(found) => {
                        if !found {
                            let mut players = players_clone.lock().unwrap();
                            let player = &mut players[client_number_copy as usize];
                            player.outgoing_data.extend_from_slice(&send_chat_message(
                                SpecialPlayers::SelfPlayer as u8,
                                "".to_string(),
                                "&cUnknown command!".to_string(),
                            ));
                        }
                    }
                    Err(err) => {
                        error!("Command error: {}", err);
                        let mut players = players_clone.lock().unwrap();
                        let player = &mut players[client_number_copy as usize];
                        player.outgoing_data.extend_from_slice(&send_chat_message(
                            SpecialPlayers::SelfPlayer as u8,
                            "".to_string(),
                            "&cCommand failed".to_string(),
                        ));
                    }
                }
            });
        }
    }
    Ok(())
}
