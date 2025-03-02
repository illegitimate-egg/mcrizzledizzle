fn metadata() {
    Metadata("ping-pong", "illegitimate-egg", Version("1.0.0"))
}

fn init(players) {
    let ctx = Context();

    ctx.register_command("ping", |player, argv| {
        players.send_message(player, "pong")
    });
    ctx.register_command("foo", |player, argv| {
        players.send_message(player, "bar")
    });

    ctx
}
