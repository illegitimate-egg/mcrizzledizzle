# Version

This type is stored internally as a complete semantic version, an easy constructor is provided that converts a valid semver to a set of parts, providing a display function.

Struct signature:
```rust
struct Version {
  major: u16,
  minor: u16,
  patch: u16,
  prerelease: String,
  build: String,
};
```

Usually it would be used along with [Metadata](./Metadata.md) to setup an extension.
