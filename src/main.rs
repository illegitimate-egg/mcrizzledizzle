#![feature(coverage_attribute)]

use log::{error, info};
use simple_logger::SimpleLogger;
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

mod command;
mod config;
mod error;
mod extensions;
mod network;
mod player;
mod utils;
mod world;

use config::Config;
use error::AppError;
use extensions::{Extensions, PlayersWrapper, WorldWrapper};
use network::handle_client;
use player::{Player, SpecialPlayers};
use utils::{client_disconnect, server_identification};
use world::World;

fn main() {
    SimpleLogger::new().with_threads(true).init().unwrap();

    if let Err(err) = run() {
        error!("FATAL: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let config = Config::load()?;

    let players: [Player; 255] = core::array::from_fn(|_| Player::default());
    let players_arc = Arc::new(Mutex::new(players));

    let world_instance: World = World::load(&config.world)?;
    let world_arc = Arc::new(Mutex::new(world_instance));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    let listener = TcpListener::bind(addr)?;

    let mut thread_number: u8 = 0;

    let world_arc_clone_main_thread = Arc::clone(&world_arc);
    let world_config_clone = config.clone().world;
    ctrlc::set_handler(move || {
        println!();
        info!("SAVING");
        World::save(&world_config_clone, world_arc_clone_main_thread.clone()).unwrap(); // Fortnite save the world
        std::process::exit(0);
    })
    .expect("Error handling control C, save on exit will not work");

    let extensions = Arc::new(Extensions::init(
        PlayersWrapper::new(players_arc.clone()),
        WorldWrapper::new(world_arc.clone()),
    )?);

    info!("Server listening on {}", config.server.port);

    for stream in listener.incoming() {
        let players_arc_clone = Arc::clone(&players_arc);
        let world_arc_clone = Arc::clone(&world_arc);
        let extensions_arc_clone = Arc::clone(&extensions);
        let mut insertion_attempts: u8 = 0;
        while players_arc.lock()?[thread_number as usize].id != SpecialPlayers::SelfPlayer as u8
            && insertion_attempts < config.server.max_players
        {
            insertion_attempts += 1;
            // One is reserved for communications
            if thread_number < config.server.max_players - 1 {
                thread_number += 1;
            } else {
                thread_number = 0;
            }
        }
        if insertion_attempts == config.server.max_players {
            // Server must be full
            // Seems silly that we have to ident to kick clients, but I didn't make the protocol
            let mut disconnect_packets: Vec<u8> = Vec::new();
            disconnect_packets
                .extend_from_slice(&server_identification(config.server.clone(), false));
            disconnect_packets
                .extend_from_slice(&client_disconnect("Server is full! Try again later"));
            stream?.write_all(&disconnect_packets)?;
        } else {
            handle_client(
                config.clone().server,
                stream?,
                thread_number,
                players_arc_clone,
                world_arc_clone,
                extensions_arc_clone,
            );
        }
    }
    Ok(())
}
