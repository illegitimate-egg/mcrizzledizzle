use log::{error, info};
use regex::Regex;
use rhai::{CustomType, Engine, EvalAltResult, FnPtr, Scope, TypeBuilder, AST};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    net::TcpStream,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    error::AppError,
    player::Player,
    utils::{send_chat_message, write_chat_stream},
};

pub struct Extensions {
    extensions: Vec<Extension>,
}

impl Extensions {
    pub fn run_command(
        &self,
        key: String,
        player: u8,
        stream: &mut TcpStream,
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
            let _ = write_chat_stream(stream, "Extension listing".to_string());

            for extension in &self.extensions {
                let _ = write_chat_stream(
                    stream,
                    format!(
                        "&a{} &bv{}",
                        extension.metadata.name,
                        extension.metadata.version.display()
                    ),
                );
            }

            return Ok(true);
        }

        // Reserve command listing command
        if &key == "commands" {
            let _ = write_chat_stream(stream, "Command listing".to_string());

            for extension in &self.extensions {
                for command in extension.commands.keys() {
                    let _ = write_chat_stream(
                        stream,
                        format!(
                            "&c{} &a[{}]",
                            command,
                            extension.metadata.name
                        ),
                    );
                }
            }

            return Ok(true);
        }

        for extension in &self.extensions {
            if let Some(key_value) = extension.commands.get(&key) {
                key_value.call::<()>(&extension.engine, &extension.ast, (player,))?;
                return Ok(true);
            }
        }

        Ok(false)
    }
}

pub struct Extension {
    ast: AST,
    engine: Engine,
    commands: HashMap<String, FnPtr>,
    metadata: ExtensionMetadata,
}

////// BEGIN RHAI DEFINITIONS //////
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

#[derive(Debug, Clone, Eq, PartialEq, CustomType)]
#[rhai_type(name = "Metadata", extra = Self::build_extra)]
struct ExtensionMetadata {
    name: String,
    version: Version,
}

impl ExtensionMetadata {
    fn new(name: String, version: Version) -> Self {
        Self { name, version }
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

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("send_message", Self::send_message);
    }
}

#[derive(Debug, Clone, CustomType)]
#[rhai_type(name = "Context", extra = Self::build_extra)]
struct Context {
    #[rhai_type(skip)]
    commands: HashMap<String, FnPtr>,
}

impl Context {
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    fn register_command(&mut self, name: String, callback: FnPtr) {
        self.commands.insert(name, callback);
    }

    fn build_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("Context", Self::new);
        builder.with_fn("register_command", Self::register_command);
    }
}
////// END RHAI DEFINITIONS //////

impl Extensions {
    pub fn init(players: PlayersWrapper) -> Result<Extensions, AppError> {
        if !Path::new("./extensions/").exists() {
            let _ = fs::create_dir("./extensions/");
        }

        let extensions_listing = fs::read_dir("./extensions")?;

        let mut extensions = Extensions {
            extensions: Vec::new(),
        };

        for extension in extensions_listing {
            let extension_path = extension?.path();

            if extension_path.extension() != Some(OsStr::new("rhai")) {
                break;
            }
            info!("Loading extension {}", extension_path.display());

            let mut engine = Engine::new();
            engine.build_type::<Version>();
            engine.build_type::<ExtensionMetadata>();
            engine.build_type::<RhaiPlayer>();
            engine.build_type::<PlayersWrapper>();
            engine.build_type::<Context>();

            let ast = engine.compile_file(extension_path.clone().into())?;
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
                        break;
                    }
                };

            let mut current_extension = Extension {
                ast,
                engine,
                commands: HashMap::new(),
                metadata: extension_metadata,
            };

            let ctx = match current_extension.engine.call_fn::<Context>(
                &mut scope,
                &current_extension.ast,
                "init",
                (PlayersWrapper::new(players.0.clone()),),
            ) {
                Ok(result) => result,
                Err(error) => {
                    error!(
                        "Plugin {} failed to init: {}",
                        current_extension.metadata.name, error
                    );
                    break;
                }
            };

            for (key, value) in ctx.commands.iter() {
                current_extension
                    .commands
                    .insert(key.to_string(), value.clone());
            }

            info!(
                "Loaded {} v{}",
                current_extension.metadata.name,
                current_extension.metadata.version.display()
            );

            extensions.extensions.push(current_extension);
        }

        for extension in &extensions.extensions {
            for command_name in extension.commands.keys() {
                info!(
                    "Extension {} v{} has reserved command: {}",
                    extension.metadata.name,
                    extension.metadata.version.display(),
                    command_name
                );
            }
        }

        Ok(extensions)
    }
}
