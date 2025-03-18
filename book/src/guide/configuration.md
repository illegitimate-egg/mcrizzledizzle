# Configuration

The config file is a regular toml file that is placed in the same directory that the program is run from. There's an example [in the repo](https://github.com/illegitimate-egg/mcrizzledizzle/blob/master/rte/config.toml) where all the available parameters have been set.

The config is split into two parts, one relating to server operations and another relating to world operations.

## [server]
Under server you can set the port, motd and name of the server.

```toml
name = "server of ire"
motd = "Message of the day" # There's a 64 character limit on these so be careful (including colour escapes)
port = 25565 # default mc port
max_players = 255 # 255 is the maximum number
```

## [world]
For the world you can set the path and size (for generation) of the world.

```toml
world = "world.wrld" # This is a custom world format that will not work with other servers

# Generator settings
size_x = 64
size_y = 32
size_z = 64
```
