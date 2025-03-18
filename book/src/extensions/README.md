# Extensions

<div class="warning">
This guide is intended for experienced programmers, it is not a step by step guide. If in doubt check the <a href="https://github.com/illegitimate-egg/mcrizzledizzle/tree/master/rte/extensions">official plugins</a>.
</div>

The extensions interface uses the [rhai](https://rhai.rs/) programming language with some custom functionality to provide a full set of instructions for making extensions.

## Extension Structure

All extensions must provide a `metadata()` and `init(players, world)` function. mcrizzledizzle uses these internally to populate information about the extension as well as register any commands or event listeners that the extension might want to use.

Example:
```rust

fn metadata() {
  Metadata("My Awesome Plugin's name!", "My Plugin's (less awesome) author", Version("1.0.0"))
}

fn init(players, world) {
  ...
}
```

The `Metadata` struct expects a name string, description string and valid `Version()`. The Version should be a valid [semantic version](https://semver.org/) otherwise the plugin will not load correctly.
