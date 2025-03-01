fn metadata() {
    Metadata("ping pong", Version("1.0.0"))
}

fn init() {
    let ctx = Context();
    ctx.register_command("ping", |player| player.sendMessage("Pong! (you smell)"));
    ctx.register_command("test", |player| player.sendMessage("test"));
    ctx
}
