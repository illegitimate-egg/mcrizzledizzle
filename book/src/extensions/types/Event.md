# Event

Events are structs that tell the extension about changes in server or player state.

```rust
struct Event {
  player: u8,
  position: Vec3,
  selected_block: u8,
  is_cancelled: bool,
}
```

Only certain parts of the struct are used for certain events. Since events are only returned to their corresponding handler, the event type is not provided in the struct. Available event types are:

- `"block_break"` This is fired when a black is broken. This is interruptible.
- `"player_leave"` This is fired when a player leaves the server. This is not interruptible.

## Implementations
### cancel
```rust
fn cancel(&mut self)
```

This sets is_cancelled to true, cancelling interruptible events, like block breaking.
