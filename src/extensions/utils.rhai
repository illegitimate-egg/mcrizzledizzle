fn metadata() {
    Metadata("utils", "illegitimate-egg", Version("1.2.3-testp+testb")) // Version test util
}

fn init(players, world) {
    let ctx = Context();

    ctx.register_command("args", |player, argv| {
        players.send_message(player, "" + argv)
    });

    ctx
}
