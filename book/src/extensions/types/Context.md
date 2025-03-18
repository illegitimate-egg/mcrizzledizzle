# Context

The context is a struct that you can create to tell the server about what your plugin wants to do. You must create one if you want to register commands or add event listeners.

```rust
struct Context {
  commands: HashMap<String, FnPtr>,
  event_listener: HashMap<EventType, FnPtr>,
};
```

## Implementations

### register_command
```rust
fn register_command(&mut self, name: String, callback: FnPtr)
```

This is how you can register commands on the server. Your `callback`` likely should be a closure as you almost certainly want to capture information from the environment.

### add_event_listener
```rust
fn add_event_listener(&mut self, event: &str, callback: FnPtr)
```

This is how event listeners are created. `callback`` should probably be a closure because you almost certainly want information from the environment. The currently available event types are:

- `"block_break"` This is fired when a black is broken. This is interruptible.
- `"player_leave"` This is fired when a player leaves the server. This is not interruptible.
