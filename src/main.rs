use log::{error, info};
use simple_logger::SimpleLogger;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

mod command;
mod error;
mod extensions;
mod network;
mod player;
mod utils;
mod world;

use error::AppError;
use extensions::{Extensions, PlayersWrapper, WorldWrapper};
use network::handle_client;
use player::{Player, SpecialPlayers};
use world::World;

fn main() {
    SimpleLogger::new().with_threads(true).init().unwrap();

    if let Err(err) = run() {
        error!("FATAL: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let players: [Player; 255] = core::array::from_fn(|_| Player::default());
    let players_arc = Arc::new(Mutex::new(players));

    let world_instance: World = World::load()?;
    let world_arc = Arc::new(Mutex::new(world_instance));

    let addr = SocketAddr::from(([0, 0, 0, 0], 25565));
    let listener = TcpListener::bind(addr)?;

    let mut thread_number: u8 = 0;

    let world_arc_clone_main_thread = Arc::clone(&world_arc);
    ctrlc::set_handler(move || {
        println!("");
        info!("SAVING");
        let _ = World::save(world_arc_clone_main_thread.clone()).unwrap(); // Fortnite save the world
        std::process::exit(0);
    })
    .expect("Error handling control C, save on exit will not work");

    let extensions = Arc::new(Extensions::init(
        PlayersWrapper::new(players_arc.clone()),
        WorldWrapper::new(world_arc.clone()),
    )?);

    info!("Server listening on {}", 25565);

    for stream in listener.incoming() {
        let players_arc_clone = Arc::clone(&players_arc);
        let world_arc_clone = Arc::clone(&world_arc);
        let extensions_arc_clone = Arc::clone(&extensions);
        handle_client(
            stream?,
            thread_number,
            players_arc_clone,
            world_arc_clone,
            extensions_arc_clone,
        );
        thread_number = thread_number.wrapping_add(1);
    }
    Ok(())
}
