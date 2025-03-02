fn metadata() {
    Metadata("ping-pong", Version("1.0.0"))
}

fn init(players) {
    let ctx = Context(players);

    ctx.register_command("ping", |player| {
        players.send_message(player, "pong")
    });
    ctx.register_command("foo", |player| {
        players.send_message(player, "bar")
    });

    ctx
}
