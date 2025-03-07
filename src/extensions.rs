use log::{debug, error, info, warn};
use regex::Regex;
use rhai::{packages::Package, CustomType, Engine, EvalAltResult, FnPtr, Scope, TypeBuilder, AST};
use rhai_rand::RandomPackage;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fmt, fs,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    error::AppError,
    player::Player,
    utils::{send_chat_message, stream_write_short, write_chat_stream},
    world::World,
};

pub struct Extensions {
    extensions: Vec<Extension>,
    players: PlayersWrapper,
}

impl Extensions {
    pub fn run_command(
        &self,
        key: String,
        player: u8,
        argv: Vec<String>,
    ) -> Result<bool, AppError> {
        // Here I'm calling write_chat_stream multiple times. This is because the stock minecraft
        // chat has a length limit of 64 characters, which is pathetically small. There is a
        // classic extension to support an unlimited number of characters, but it's not guaranteed
        // that the client will support it, so the next best option is to just send multiple
        // messages, they're newline seperated anyway. I am aware that repeated stream writes are
        // not the best option however, and that at some point I should switch to buffered streams.
        // TODO: Use buffered streams (That's everywhere not just here)

        // Reserve extension listing command
        if &key == "extensions" {
            let mut res_data: Vec<u8> = Vec::new();
            res_data.extend_from_slice(&write_chat_stream("Extension listing".to_string()));

            for extension in &self.extensions {
                res_data.extend_from_slice(&write_chat_stream(format!(
                    "&a{} &bv{}",
                    extension.metadata.name, extension.metadata.version
                )));
            }

            self.players.0.lock().unwrap()[player as usize]
                .outgoing_data
                .extend_from_slice(&res_data);

            return Ok(true);
        }

        // Reserve command listing command
        if &key == "help" {
            let mut res_data: Vec<u8> = Vec::new();

            res_data.extend_from_slice(&write_chat_stream("Command listing".to_string()));

            res_data.extend_from_slice(&write_chat_stream(format!(
                "&c{} &a[{}]",
                "help", "Builtin"
            )));
            res_data.extend_from_slice(&write_chat_stream(format!(
                "&c{} &a[{}]",
                "extensions", "Builtin"
            )));
            res_data.extend_from_slice(&write_chat_stream(format!(
                "&c{} &a[{}]",
                "kick", "Builtin"
            )));
            res_data.extend_from_slice(&write_chat_stream(format!("&c{} &a[{}]", "tp", "Builtin")));

            for extension in &self.extensions {
                for command in extension.commands.keys() {
                    res_data.extend_from_slice(&write_chat_stream(format!(
                        "&c{} &a[{}]",
                        command, extension.metadata.name
                    )));
                }
            }

            self.players.0.lock().unwrap()[player as usize]
                .outgoing_data
                .extend_from_slice(&res_data);

            return Ok(true);
        }

        for extension in &self.extensions {
            if let Some(key_value) = extension.commands.get(&key) {
                // Vector transfer number 2 (get the shit joke?)
                let mut argv_rhai_array: rhai::Array = rhai::Array::new();
                for arg in &argv {
                    argv_rhai_array.push(arg.into());
                }

                key_value.call::<()>(
                    &extension.engine,
                    &extension.ast,
                    (player, argv_rhai_array),
                )?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn run_event(&self, event_type: EventType, event: Event) -> Event {
        let mut is_cancelled = false;

        for extension in &self.extensions {
            if let Some(key_value) = extension.event_listeners.get(&event_type) {
                is_cancelled =
                    match key_value.call::<Event>(&extension.engine, &extension.ast, (event,)) {
                        Ok(result) => result.is_cancelled,
                        Err(err) => {
                            error!("{} raised: {}", extension.metadata.name, err);
                            break;
                        }
                    };
            }
        }
        let mut response = Event::new();
        response.is_cancelled = is_cancelled;
        response
    }
}

pub struct Extension {
    ast: AST,
    engine: Engine,
    commands: HashMap<String, FnPtr>,
    event_listeners: HashMap<EventType, FnPtr>,
    metadata: ExtensionMetadata,
}

////// BEGIN RHAI DEFINITIONS //////
fn info(msg: &str) {
    info!("{}", msg);
}

fn warn(msg: &str) {
    warn!("{}", msg);
}

fn error(msg: &str) {
    error!("{}", msg);
}

fn debug(msg: &str) {
    debug!("{}", msg);
}

#[derive(Debug, Clone, Eq, PartialEq, CustomType)]
#[rhai_type(name = "Version", extra = Self::build_extra)]
struct Version {
    major: u16,
    minor: u16,
    patch: u16,
    prerelease: String,
    build: String,
}

impl Version {
    pub fn display(&self) -> String {
        let mut base = format!("{}.{}.{}", self.major, self.minor, self.patch);

        if !self.prerelease.is_empty() {
            base.push_str(&format!("-{}", self.prerelease).to_string());
        }

        if !self.build.is_empty() {
            base.push_str(&format!("+{}", self.build).to_string());
        }

        base
    }

    fn parse(version_string: String) -> Result<Self, Box<EvalAltResult>> {
        let Ok(re) = Regex::new(
            r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$",
        ) else {
            return Err("Failed to create regex".into());
        };

        let Some(version_parts) = re.captures(&version_string) else {
            return Err("Invalid Extension Version".into());
        };

        let mut prerelease: String = "".to_string();
        let mut build: String = "".to_string();

        if version_parts.name("prerelease").is_some() {
            prerelease = version_parts["prerelease"].to_string();
        }
        if version_parts.name("buildmetadata").is_some() {
            build = version_parts["buildmetadata"].to_string();
        }

        Ok(Version {
            major: version_parts["major"].parse::<u16>().unwrap(),
            minor: version_parts["minor"].parse::<u16>().unwrap(),
            patch: version_parts["patch"].parse::<u16>().unwrap(),
            prerelease,
            build,
        })
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        // Register constructor function
        builder.with_fn("Version", Self::parse);
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, CustomType)]
#[rhai_type(name = "Metadata", extra = Self::build_extra)]
struct ExtensionMetadata {
    name: String,
    author: String,
    version: Version,
}

impl ExtensionMetadata {
    fn new(name: String, author: String, version: Version) -> Self {
        Self {
            name,
            author,
            version,
        }
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        // Register constructor function
        builder.with_fn("Metadata", Self::new);
    }
}

#[derive(Debug, Clone, CustomType)]
#[rhai_type(name = "Player")]
struct RhaiPlayer {
    id: u8,
}

#[derive(Debug, Clone, CustomType)]
#[rhai_type(name = "PlayersWrapper", extra=Self::build_extra)]
pub struct PlayersWrapper(Arc<Mutex<[Player; 255]>>);

impl PlayersWrapper {
    pub fn new(players: Arc<Mutex<[Player; 255]>>) -> Self {
        Self(players)
    }

    fn send_message(self, player: u8, message: String) -> Result<(), Box<EvalAltResult>> {
        let mut players = self.0.lock().unwrap();

        players[player as usize]
            .outgoing_data
            .extend_from_slice(&send_chat_message(255, "".to_string(), message));

        Ok(())
    }

    fn send_all(self, message: String) -> Result<(), Box<EvalAltResult>> {
        let mut players = self.0.lock().unwrap();

        let data = &send_chat_message(255, "".to_string(), message);

        for i in 0..255 {
            if players[i].id != 255 {
                players[i].outgoing_data.extend_from_slice(data);
            }
        }

        Ok(())
    }

    fn username(self, player: u8) -> String {
        let players = self.0.lock().unwrap();

        players[player as usize].username.clone()
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("send_message", Self::send_message);
        builder.with_fn("send_all", Self::send_all);
        builder.with_fn("username", Self::username);
    }
}

#[derive(Debug, Clone, CustomType)]
#[rhai_type(name = "WorldWrapper", extra=Self::build_extra)]
pub struct WorldWrapper(Arc<Mutex<World>>);

impl WorldWrapper {
    pub fn new(world: Arc<Mutex<World>>) -> Self {
        Self(world)
    }

    pub fn set_block(self, players_wrapper: PlayersWrapper, position: Vec3, block_type: u8) {
        let mut world_dat = self.0.lock().unwrap();

        let world_offset: u32 = position.x as u32
            + (position.z as u32 * world_dat.size_x as u32)
            + (position.y as u32 * world_dat.size_x as u32 * world_dat.size_z as u32);

        world_dat.data[world_offset as usize] = block_type;

        let mut update_block_bytes: Vec<u8> = Vec::new();
        update_block_bytes.push(0x06);
        update_block_bytes.extend_from_slice(&stream_write_short(position.x));
        update_block_bytes.extend_from_slice(&stream_write_short(position.y));
        update_block_bytes.extend_from_slice(&stream_write_short(position.z));
        update_block_bytes.push(block_type);

        let mut players = players_wrapper.0.lock().unwrap();
        for i in 0..players.len() {
            if players[i].id != 255 {
                players[i]
                    .outgoing_data
                    .extend_from_slice(&update_block_bytes);
            }
        }
    }

    // TODO: Finish this
    // pub fn get_block(&self, position: Vec3) -> u8 {
    //     let mut world = self.0.lock().unwrap();
    // }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("set_block", Self::set_block);
        // builder.with_fn("get_block", Self::get_block);
    }
}

#[derive(Debug, Clone, CustomType)]
#[rhai_type(name = "Context", extra = Self::build_extra)]
struct Context {
    #[rhai_type(skip)]
    commands: HashMap<String, FnPtr>,
    #[rhai_type(skip)]
    event_listener: HashMap<EventType, FnPtr>,
}

#[derive(Debug, Clone, Copy, PartialEq, CustomType)]
#[rhai_type(name = "Vec3", extra = Self::build_extra)]
pub struct Vec3 {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

// Custom type API
impl Vec3 {
    fn new(x: i64, y: i64, z: i64) -> Self {
        Self {
            x: x.try_into().unwrap(),
            y: y.try_into().unwrap(),
            z: z.try_into().unwrap(),
        }
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("Vec3", Self::new);
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum EventType {
    BlockBreak,
    PlayerLeave,
}

#[derive(Debug, Clone, Copy, CustomType)]
#[rhai_type(name = "Event", extra = Self::build_extra)]
pub struct Event {
    pub player: u8,
    pub position: Vec3,
    pub selected_block: u8,
    pub is_cancelled: bool,
}

impl Event {
    pub fn new() -> Event {
        Event {
            player: 255,
            position: (Vec3 { x: 0, y: 0, z: 0 }),
            selected_block: 0,
            is_cancelled: false,
        }
    }

    pub fn cancel(&mut self) {
        self.is_cancelled = true;
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("Event", Self::new);
        builder.with_fn("cancel", Self::cancel);
    }
}

impl Context {
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
            event_listener: HashMap::new(),
        }
    }

    fn register_command(&mut self, name: String, callback: FnPtr) {
        self.commands.insert(name, callback);
    }

    fn add_event_listener(&mut self, event: &str, callback: FnPtr) {
        let event_listener: EventType = match event {
            "block_break" => EventType::BlockBreak,
            "player_leave" => EventType::PlayerLeave,
            _ => {
                warn!("An event listener was created with invalid type: {}", event);
                return;
            }
        };
        self.event_listener.insert(event_listener, callback);
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("Context", Self::new);
        builder.with_fn("register_command", Self::register_command);
        builder.with_fn("add_event_listener", Self::add_event_listener);
    }
}
////// END RHAI DEFINITIONS //////

impl Extensions {
    pub fn init(players: PlayersWrapper, world: WorldWrapper) -> Result<Extensions, AppError> {
        if !Path::new("./extensions/").exists() {
            let _ = fs::create_dir("./extensions/");
        }

        let extensions_listing = fs::read_dir("./extensions")?;

        let mut extensions = Extensions {
            extensions: Vec::new(),
            players: players.clone(),
        };

        for extension in extensions_listing {
            let extension_path = extension?.path();

            if extension_path.extension() != Some(OsStr::new("rhai")) {
                break;
            }
            info!("Loading extension {}", extension_path.display());

            let mut engine = Engine::new();
            let random = RandomPackage::new();
            random.register_into_engine(&mut engine);
            engine.set_max_expr_depths(50, 50);
            engine.build_type::<Version>();
            engine.build_type::<ExtensionMetadata>();
            engine.build_type::<RhaiPlayer>();
            engine.build_type::<PlayersWrapper>();
            engine.build_type::<WorldWrapper>();
            engine.build_type::<Context>();
            engine.build_type::<Vec3>();
            engine.build_type::<Event>();
            engine.register_fn("info", info);
            engine.register_fn("warn", warn);
            engine.register_fn("error", error);
            engine.register_fn("debug", debug);

            let ast = match engine.compile_file(extension_path.clone().into()) {
                Ok(result) => result,
                Err(error) => {
                    error!(
                        "Rhai plugin compilation failed for {}, reason: {}",
                        extension_path.display(),
                        error
                    );
                    continue;
                }
            };
            let mut scope = Scope::new();

            let extension_metadata =
                match engine.call_fn::<ExtensionMetadata>(&mut scope, &ast, "metadata", ()) {
                    Ok(result) => result,
                    Err(error) => {
                        error!(
                            "Rhai plugin with path {} missing critical section metadata! {}",
                            extension_path.display(),
                            error
                        );
                        continue;
                    }
                };

            let mut current_extension = Extension {
                ast,
                engine,
                commands: HashMap::new(),
                event_listeners: HashMap::new(),
                metadata: extension_metadata,
            };

            let ctx = match current_extension.engine.call_fn::<Context>(
                &mut scope,
                &current_extension.ast,
                "init",
                (
                    PlayersWrapper::new(players.0.clone()),
                    WorldWrapper::new(world.0.clone()),
                ),
            ) {
                Ok(result) => result,
                Err(error) => {
                    error!(
                        "Plugin {} failed to init: {}",
                        current_extension.metadata.name, error
                    );
                    continue;
                }
            };

            for (key, value) in ctx.commands.iter() {
                current_extension
                    .commands
                    .insert(key.to_string(), value.clone());
            }

            for (key, value) in ctx.event_listener.iter() {
                current_extension
                    .event_listeners
                    .insert(key.clone(), value.clone());
            }

            info!(
                "Loaded {} v{}",
                current_extension.metadata.name, current_extension.metadata.version,
            );

            extensions.extensions.push(current_extension);
        }

        for extension in &extensions.extensions {
            for command_name in extension.commands.keys() {
                info!(
                    "Extension {} v{} has reserved command: {}",
                    extension.metadata.name, extension.metadata.version, command_name
                );
            }
        }

        Ok(extensions)
    }
}
