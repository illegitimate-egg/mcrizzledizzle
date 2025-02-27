#[derive(Debug)]
pub struct Player {
    // Struct `Player` is never constructed `#[warn(fuck_you)]` on by default
    pub id: u8,
    pub username: String,
    pub verification_key: [u8; 64],
    pub unused: u8,
    pub position_x: i16,
    pub position_y: i16,
    pub position_z: i16,
    pub yaw: u8,
    pub pitch: u8,
    pub operator: bool,
    pub outgoing_data: Vec<u8>,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            id: SpecialPlayers::SelfPlayer as u8,
            username: "".to_string(),
            verification_key: [0; 64],
            unused: 0x00,
            position_x: 0,
            position_y: 0,
            position_z: 0,
            yaw: 0,
            pitch: 0,
            operator: false,
            outgoing_data: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum SpecialPlayers {
    SelfPlayer = 0xFF,
}

#[derive(Copy, Clone, PartialEq)]
pub enum PlayerStatus {
    Disconnected,
    ConnectedSelf,
    Connected,
}
