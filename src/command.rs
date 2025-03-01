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
    let vectorized_command = command_string.split(" ").collect::<Vec<&str>>();
    match vectorized_command[0] {
        "kick" => {
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
        "tp" => {
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
        _ => {
            let found = match extensions.run_command(vectorized_command[0].to_string(), client_number) {
                Ok(result) => result,
                Err(error) => {
                    error!("Rhai plugin error: {}", error);
                    let _ = &mut stream.write(&send_chat_message(
                        SpecialPlayers::SelfPlayer as u8,
                        "".to_string(),
                        "&cAn internal error occured while processing this command".to_string(),
                    ));
                    return Ok(());
                }
            };

            if found {
                return Ok(());
            }

            let _ = &mut stream.write(&send_chat_message(
                SpecialPlayers::SelfPlayer as u8,
                "".to_string(),
                "&cUnkown command!".to_string(),
            ));
        }
    }
    Ok(())
}
