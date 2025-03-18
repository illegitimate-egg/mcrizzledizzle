# Metadata

This type tells mcrizzledizzle important imformation about the plugin.

The struct signature looks like this:

```rust
struct Metadata {
  name: String,
  author: String,
  version: Version,
};
```

It should be used inside the metadata function like so:

```rust
fn metadata() {
  Metadata("Example Name", "Example Description", Version("1.0.0"))
}
```
