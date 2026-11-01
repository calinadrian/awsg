//! ASR's room directory and match relay. Gameplay belongs to the player hosting the room.
use serde::{Deserialize, Serialize};
pub mod economy;
#[cfg(not(target_arch = "wasm32"))]
pub mod economy_store;

pub const PROTOCOL: u32 = 1;
pub const BUILD: &str = "asr-elwynn-coop-economy-3";
pub const MAX_PLAYERS: usize = 4;
pub const MAX_MESSAGE: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hero {
    pub class: usize,
    pub race: u8,
    pub sex: u8,
    pub display: u32,
    /// The menu's appearance dials `[skin, face, hair style, hair colour, facial hair]`; absent
    /// from older clients, whose heroes wear their display's own look.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<[u8; 5]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub peer: u32,
    pub name: String,
    pub hero: Hero,
    pub ready: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub code: String,
    pub name: String,
    pub map: usize,
    pub private: bool,
    pub members: Vec<Member>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Client {
    Economy {
        request: economy::Request,
    },
    Hello {
        protocol: u32,
        build: String,
    },
    List,
    Create {
        name: String,
        private: bool,
        map: usize,
        member: Member,
    },
    Join {
        code: String,
        member: Member,
    },
    Ready {
        ready: bool,
    },
    Start,
    Leave,
    /// Guest data goes only to the host. Host data goes only to this room's guests.
    Data {
        payload: serde_json::Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Server {
    Economy {
        reply: economy::Reply,
    },
    Welcome,
    Rooms {
        rooms: Vec<Room>,
    },
    Room {
        room: Room,
        peer: u32,
    },
    Start,
    Data {
        peer: u32,
        payload: serde_json::Value,
    },
    Left {
        peer: u32,
    },
    Closed {
        reason: String,
    },
    Error {
        reason: String,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub mod transport;
