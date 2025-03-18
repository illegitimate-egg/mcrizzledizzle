# PlayersWrapper

The PlayersWrapper struct wraps around the server's Player Data mutex. It provides a friendly interface for extensions.

```rust
struct PlayersWrapper(Arc<Mutex<[Player; 255]>>);
```

You cannot instantiate a PlayersWrapper yourself, it is passed as the first argument of init() and can then be called upon from there.

## Implementations

### send_message
```rust
fn send_message(
  self,
  player: u8,
  message: String,
)
```

This function expects a player id and a message to be passed as arguments. Keep in mind that the length limit for messages is 64 characters.

### send_all
```rust
fn send_all(self, message: String)
```

This function is like `send_message` except it sends the message to all connected players.

### username
```rust
fn username(self, player: u8)
```

This function gets the username of a player from their id.
