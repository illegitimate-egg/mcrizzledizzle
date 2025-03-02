fn metadata() {
    Metadata("jimmy", Version("2.0.0-balls+tweaking"))
}

fn init(players) {
    let ctx = Context();
    ctx.register_command("jimmy", |player| {
        players.send_message(player, "He's dead")
    });
    ctx
}
