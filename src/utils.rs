use flate2::write::GzEncoder;
use flate2::Compression;
use log::info;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::config::ServerConfig;
use crate::error::AppError;
use crate::Player;
use crate::SpecialPlayers;
use crate::World;

pub fn to_mc_string(text: &str) -> [u8; 64] {
    let text_vec: Vec<char> = text.chars().take(64).collect();
    let mut balls = [0x20; 64];

    for i in 0..text_vec.len() {
        balls[i] = text_vec[i] as u8;
    }

    balls
}

pub fn stream_write_short(data: i16) -> Vec<u8> {
    [(data >> 0x08) as u8, (data & 0x00FF) as u8].to_vec()
}

pub fn client_disconnect(text: &str) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x0E);
    ret_val.append(&mut to_mc_string(text).to_vec());
    ret_val
}

pub fn server_identification(config: ServerConfig, is_op: bool) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x00);
    ret_val.push(0x07);

    ret_val.append(&mut to_mc_string(&config.name).to_vec());
    ret_val.append(&mut to_mc_string(&config.motd).to_vec());

    if is_op {
        ret_val.push(0x64);
    } else {
        ret_val.push(0x00);
    }

    ret_val
}

pub fn ping() -> Vec<u8> {
    vec![0x01]
}

pub fn init_level() -> Vec<u8> {
    vec![0x02]
}

pub fn finalize_level(world_arc_clone: &Arc<Mutex<World>>) -> Result<Vec<u8>, AppError> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x04);

    let world_dat = world_arc_clone.lock()?;

    ret_val.append(&mut stream_write_short(world_dat.size_x).to_vec());
    ret_val.append(&mut stream_write_short(world_dat.size_y).to_vec());
    ret_val.append(&mut stream_write_short(world_dat.size_z).to_vec());

    Ok(ret_val)
}

pub fn spawn_player(
    player_id: u8,
    name: &str,
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

    ret_val
}

pub fn despawn_player(player_id: u8) -> Vec<u8> {
    [0x0C, player_id].to_vec()
}

pub fn send_chat_message(
    source_id: u8,
    mut source_username: String,
    mut message: String,
) -> Vec<u8> {
    let mut ret_val: Vec<u8> = vec![];
    ret_val.push(0x0D);

    if !source_username.is_empty() {
        source_username.push_str(": ");
    }
    message.insert_str(0, &source_username);

    ret_val.push(source_id);
    ret_val.append(&mut to_mc_string(&message).to_vec());

    ret_val
}

pub fn write_chat_stream(message: String) -> Vec<u8> {
    send_chat_message(SpecialPlayers::SelfPlayer as u8, "".to_string(), message)
}

pub fn set_position_and_orientation(
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

    ret_val
}

pub fn send_level_data(world_arc_clone: &Arc<Mutex<World>>) -> Result<Vec<u8>, AppError> {
    let mut ret_val: Vec<u8> = vec![];
    let mut world_dat = world_arc_clone.lock()?.data.clone();

    // Big endian fold lmao
    world_dat.insert(0, (world_dat.len() & 0xFF) as u8);
    world_dat.insert(0, ((world_dat.len() & 0xFF00) >> 8) as u8);
    world_dat.insert(0, ((world_dat.len() & 0xFF0000) >> 16) as u8);
    world_dat.insert(0, ((world_dat.len() & 0xFF000000) >> 24) as u8);

    // TODO: Stream GZIP straight onto the network

    let mut world_dat_compressor = GzEncoder::new(Vec::new(), Compression::fast());
    let _ = world_dat_compressor.write_all(&world_dat);
    let world_dat_gzipped = world_dat_compressor.finish()?;

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

            ret_val.push(percentage.try_into()?);

            current_chunk += 1;
        }
    }

    let remaining_chunk_size = world_dat_gzipped.len() - (current_chunk * 1024);

    if remaining_chunk_size > 0 {
        ret_val.push(0x03);

        ret_val.append(&mut stream_write_short(remaining_chunk_size.try_into()?));

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
    Ok(ret_val)
}

pub fn bomb_server_details(
    config: ServerConfig,
    stream: &mut TcpStream,
    current_player: &Player,
    world_arc_clone: &Arc<Mutex<World>>,
) -> Result<(), AppError> {
    let mut compound_data: Vec<u8> = vec![];
    compound_data.append(&mut server_identification(config, current_player.operator));

    compound_data.append(&mut init_level());

    // info!("Send level data");
    compound_data.append(&mut send_level_data(world_arc_clone)?); // Approaching Nirvana - Maw of the beast

    compound_data.append(&mut finalize_level(world_arc_clone)?);

    info!("Spawning player: {}", &current_player.username);
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_writer() {
        assert_eq!(to_mc_string("This is a nice test string that's longer than 64 chars. By having a string of this length it checks if to_mc_string truncates text correctly."), [84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 110, 105, 99, 101, 32, 116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 116, 104, 97, 116, 39, 115, 32, 108, 111, 110, 103, 101, 114, 32, 116, 104, 97, 110, 32, 54, 52, 32, 99, 104, 97, 114, 115, 46, 32, 66, 121, 32, 104, 97, 118, 105, 110]);
    }

    #[test]
    fn test_short_writer() {
        for x in i16::MIN..i16::MAX {
            assert_eq!(
                stream_write_short(x),
                [(x.to_le() >> 0x08) as u8, (x.to_le() & 0x00FF) as u8].to_vec() // There is a very
                                                                                 // real argument that I can't counter that says this is how this function should be
                                                                                 // implemented, and also that you can't test a range of data but to that I say...
                                                                                 // yeah ig TODO:
                                                                                 // Make this not a pile of shit
            );
        }
    }

    #[test]
    fn test_client_disconnect() {
        assert_eq!(
            client_disconnect("test string"),
            [
                14, 116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32
            ]
        );
    }
}
