fn metadata() {
    Metadata("jimmy", Version("2.0.0-balls+tweaking"))
}

fn init(players) {
    let ctx = Context(players);
    ctx.register_command("jimmy", |player| {
        players.send_message("He's dead")
    });
    ctx
}
