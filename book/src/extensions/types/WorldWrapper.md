# WorldWrapper

The WorldWrapper struct wraps around the server's World Data mutex. It provides a friendly interface for extensions.

```rust
struct WorldWrapper(Arc<Mutex<World>>);
```

You cannot instantiate a WorldWrapper yourself, it is passed as the second argument of init() and can be called upon from there.

## Implementations

### set_block
```rust
fn set_block(
  self,
  players_wrapper: PlayersWrapper,
  position: Vec3,
  block_type: u8,
)
```

This functions sets a block at the desired position. Since it uses the player data internally it requires the PlayersWrapper.
