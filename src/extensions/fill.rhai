fn metadata() {
    Metadata("fill", "illegitimate-egg", Version("1.0.0"))
}

fn init(players, world) {
    let ctx = Context();

    let playerData = #{};

    ctx.register_command("fill", |player, argv| {
        players.send_message(player, "Break two blocks to select area");
        playerData[player.to_string()] = #{command_step: 1};
    });

    ctx.add_event_listener("block_break", |event| {
        let player_data = playerData[event.player.to_string()];
        if player_data != () {
            switch player_data.command_step {
                0 => {
                    // Do nothing, the command isn't in use
                }
                1 => {
                    player_data.command_step += 1;
                    event.cancel();

                    player_data.firstBlock = event.position;
                    players.send_message(event.player, "Position 1 {" + event.position.x + ", " + event.position.y + ", " + event.position.z + "}");
                }
                2 => {
                    player_data.command_step = 0;
                    event.cancel();

                    player_data.secondBlock = event.position;

                    players.send_message(event.player, "Position 2 {" + event.position.x + ", " + event.position.y + ", " + event.position.z + "}");

                    if (player_data.firstBlock.x > player_data.secondBlock.x) {
                        let buffer = player_data.firstBlock.x;
                        player_data.firstBlock.x = player_data.secondBlock.x;
                        player_data.secondBlock.x = buffer;
                    }
                    if (player_data.firstBlock.y > player_data.secondBlock.y) {
                        let buffer = player_data.firstBlock.y;
                        player_data.firstBlock.y = player_data.secondBlock.y;
                        player_data.secondBlock.y = buffer;
                    }
                    if (player_data.firstBlock.z > player_data.secondBlock.z) {
                        let buffer = player_data.firstBlock.z;
                        player_data.firstBlock.z = player_data.secondBlock.z;
                        player_data.secondBlock.z = buffer;
                    }

                    let filled_blocks = 0;

                    for x in (player_data.firstBlock.x.to_int())..=(player_data.secondBlock.x.to_int()) {
                        for y in (player_data.firstBlock.y.to_int())..=(player_data.secondBlock.y.to_int()) {
                            for z in (player_data.firstBlock.z.to_int())..=(player_data.secondBlock.z.to_int()) {
                                world.set_block(players, Vec3(x, y, z), event.selected_block);
                                filled_blocks = filled_blocks + 1;
                            }
                        }
                    }

                    players.send_message(event.player, "Filled " + filled_blocks + " blocks");
                }
                _ => {
                    error("Unreachable reached");
                }
            }
        }


        playerData[event.player.to_string()] = player_data;
        return event;
    });
    
    ctx.add_event_listener("player_leave", |event| {
      playerData[event.player.to_string()].command_step = 0;
    });

    ctx
}
