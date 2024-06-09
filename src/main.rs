use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;
use std::thread;

fn handle_client(mut stream: TcpStream, client_number: u8) {
    thread::spawn(move || {
    let mut buffer = [0; 256];
    let _ = stream.read(&mut buffer);
//    let _ = stream.write(&[8]);

    match buffer[0] {
        0x00=> {
            println!("Client Prot Ver: {}", buffer[1]);
            let mut username = String::new();

            for i in 0..64 {
                username.push(buffer[i+2] as char);
            }
            println!("Username: {}", username);

            let mut verif_key = [0; 64];

            for i in 0..64 {
                verif_key[i] = buffer[i+66];
            }

            let mut verif_key_formatted = String::new();
            use std::fmt::Write;
            for &byte in &verif_key {
                write!(&mut verif_key_formatted, "{:X}", byte).expect("Piss");
            }

            println!("Verification key: 0x{}", verif_key_formatted);

            println!("\"Unused\" Byte: {}", buffer[130]);
            bomb_server_details(stream, client_number)
        },
        _=>println!("Mindfield"),
    }
    });
}

fn to_mc_string(text: &str) -> [u8; 64] {
    let text_vec: Vec<char> = text.chars().collect();
    let mut balls = [0; 64];

    for i in 0..64 {
        balls[i] = 0x20;
    }

    for i in 0..text.len() {
        balls[i] = text_vec[i] as u8;
    }

    return balls;
}

fn bomb_server_details(mut stream: TcpStream, client_number: u8) {
    let _ = stream.write(&[0]); // Server IDENT
    println!("Server IDENT");
    let _ = stream.write(&[0x07]); // Protocol version 7
    println!("Server protocol ver: 7");

    let server_name = format!("Sigma Balls {}", client_number);
    for i in 0..64 {
        let _ = stream.write(&[to_mc_string(&server_name)[i]]); // Send server name
    }
    println!("Server name: {}", server_name);
    
    let motd = "Pragmatism not idealism";
    for i in 0..64 {
        let _ = stream.write(&[to_mc_string(&motd)[i]]); // Send server name
    }
    println!("MOTD: {}", motd);

    let _ = stream.write(&[0x64]);
    println!("Player is an OP (shits pants)");
    //let _ = stream.write(&[0x01]); // Ping the rizzler
    let _ = stream.write(&[0x02]); // Init level
    // Shit must be sent here (Shits pants)
    let _ = stream.write(&[0x04]); // Finalize level
    let _ = stream.write(&[0xFF]); // Level Size X
    let _ = stream.write(&[0xFF]); // Level Size X B2
    let _ = stream.write(&[0xFF]); // Height Limit
    let _ = stream.write(&[0xFF]); // Height Limit B2
    let _ = stream.write(&[0xFF]); // Level Size Z
    let _ = stream.write(&[0xFF]); // Level Size Z B2
    let _ = stream.write(&[0x07]); // Spawn player
    let _ = stream.write(&[0x01]); // Player ID
    let name = format!("Ultra {}", client_number);
    for i in 0..64 { // Send player name
        let _ = stream.write(&[to_mc_string(&name)[i]]);  // Send server name
    }
    let _ = stream.write(&[0x00]); // Spawn X
    let _ = stream.write(&[0x00]); // Spawn X B2
    let _ = stream.write(&[0x00]); // Spawn Y
    let _ = stream.write(&[0x00]); // Spawn Y B2
    let _ = stream.write(&[0x00]); // Spawn Z
    let _ = stream.write(&[0x00]); // Spawn Z B2
    let _ = stream.write(&[0x00]); // Spawn YAW
    let _ = stream.write(&[0x00]); // Spawn PITCH

    let _ = stream.write(&[0x01]);
    let _ = stream.write(&[0x01]);
    let _ = stream.write(&[0x01]);

    let mut mega_rizma_test_one : u8 = 1;

    loop {
        let is_kill = stream.write(&[0x01]); // Ping that MF
        
        if is_kill.is_err() {
            println!("Thread {} is kill!", client_number);
            break;
        }

        let _ = stream.write(&[0x07]); // Spawn player
        let _ = stream.write(&[mega_rizma_test_one]); // Player ID
        let nameb = format!("Brizinga {}", mega_rizma_test_one);
        for i in 0..64 { // Send player name
            let _ = stream.write(&[to_mc_string(&nameb)[i]]);  // Send server name
        }
        let _ = stream.write(&[mega_rizma_test_one << 5]); // Spawn X
        let _ = stream.write(&[0x00]); // Spawn X B2
        let _ = stream.write(&[0x00]); // Spawn Y
        let _ = stream.write(&[0x00]); // Spawn Y B2
        let _ = stream.write(&[0x00]); // Spawn Z
        let _ = stream.write(&[0x00]); // Spawn Z B2
        let _ = stream.write(&[0x00]); // Spawn YAW
        let _ = stream.write(&[0x00]); // Spawn PITCH

        let _ = stream.write(&[0x0D]);
        let _ = stream.write(&[0x01]); // Player ID
        for i in 0..64 { // Send player name
            let _ = stream.write(&[to_mc_string(&nameb)[i]]);  // Send server name
        }

        println!("Player position updated, test = {}", mega_rizma_test_one);

        if mega_rizma_test_one != 254 {
            mega_rizma_test_one += 1;
        } else {
            mega_rizma_test_one = 0;
        }

        //sleep(Duration::from_millis(10));
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.01:25565")?;
    
    let mut thread_number : u8 = 0;

    for stream in listener.incoming() {
        handle_client(stream?, thread_number);
        thread_number += 1;
    }
    Ok(())
}
