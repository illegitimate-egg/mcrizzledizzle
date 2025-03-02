fn metadata() {
    Metadata("fill", "illegitimate-egg", Version("1.0.0")) // Version test util
}

fn init(players) {
    let ctx = Context();

    let playerData = #{};

    ctx.register_command("fill", |player, argv| {
        players.send_message(player, "Break two blocks to select area");
        playerData[player].command_step == 1;
    });

    ctx.add_event_listener("block_break", |event| {
        if playerData[event.player].command_step == () {
            playerData[event.player].command_step == 0;
        } else {
            switch playerData[event.player].command_step {
                0 => {
                    // Do nothing, the command isn't in use
                }
                1 => {
                    // Record the block position
                    playerData[event.player].firstBlock = event.position;
                }
                2 => {
                    playerData[event.player].secondBlock = event.position;
                    // TODO: Implement the fill
                }
                _ => {
                    error("Unreachable reached");
                }
            }
        }
    });
    
    // NOTE: Currently this doesn't work, as player_leave isn't implemented
    ctx.add_event_listener("player_leave", |event| {
      playerData[event.player].command_step = 0;
    });

    ctx
}
