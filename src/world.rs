use std::fs::{self, File};
use std::io::Write;
use std::sync::{Arc, Mutex};

use log::info;

use crate::error::AppError;

#[derive(Debug)]
pub struct World {
    pub size_x: i16,
    pub size_y: i16,
    pub size_z: i16,
    pub data: Vec<u8>,
}

impl World {
    fn build(size_x: i16, size_y: i16, size_z: i16) -> Vec<u8> {
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

        world_dat
    }
    pub fn load() -> Result<Self, AppError> {
        if fs::metadata("world.wrld").is_ok() {
            let mut world: World = World {
                size_x: 0,
                size_y: 0,
                size_z: 0,
                data: Vec::new(),
            };

            let world_data_raw = fs::read("world.wrld")?;
            if world_data_raw.len() < 6 {
                return Err(AppError::InvalidWorldFile);
            }
            world.size_x = ((world_data_raw[0] as i16) << 8) + (world_data_raw[1] as i16);
            world.size_y = ((world_data_raw[2] as i16) << 8) + (world_data_raw[3] as i16);
            world.size_z = ((world_data_raw[4] as i16) << 8) + (world_data_raw[5] as i16);

            if world.size_x > 512 || world.size_y > 256 || world.size_z > 512 {
                return Err(AppError::InvalidWorldFile);
            }

            if world_data_raw.len()
                != (world.size_x as i32 * world.size_y as i32 * world.size_z as i32 + 6 as i32)
                    as usize
            {
                return Err(AppError::InvalidWorldFile);
            }

            world.data = world_data_raw[6..].to_vec();
            info!("Loaded world {}", "world.wrld");
            Ok(world)
        } else {
            info!("Creating word {}", "world.wrld");
            Ok(World {
                size_x: 64,
                size_y: 32,
                size_z: 64,
                data: World::build(64, 32, 64),
            })
        }
    }

    pub fn save(world_arc_clone: Arc<Mutex<World>>) -> Result<(), AppError> {
        let mut to_write: Vec<u8> = Vec::new();
        {
            let mut world_dat = world_arc_clone
                .lock()
                .map_err(|e| AppError::MutexPoisoned(e.to_string()))?;

            to_write.push((world_dat.size_x >> 8) as u8);
            to_write.push((world_dat.size_x & 0xFF) as u8);
            to_write.push((world_dat.size_y >> 8) as u8);
            to_write.push((world_dat.size_y & 0xFF) as u8);
            to_write.push((world_dat.size_z >> 8) as u8);
            to_write.push((world_dat.size_z & 0xFF) as u8);
            to_write.append(&mut world_dat.data);
        }

        let mut file = File::create("world.wrld")?;
        Ok(file.write_all(&to_write)?)
    }
}
