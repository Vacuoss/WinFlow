use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub tcp_port: u16,
    pub udp_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello(DeviceInfo),

    EnterControl {
        edge: String,
    },

    ExitControl,

    ReturnControl,

    OpenUrl {
        url: String,
    },

    MouseMove {
        dx: i32,
        dy: i32,
    },

    MouseButton {
        button: MouseButton,
        down: bool,
    },

    MouseWheel {
        delta: i32,
    },

    ClipboardText {
        text: String,
        hash: String,
    },

    DropStart {
        id: String,
        name: String,
        size: u64,
        sha256: String,
    },

    DropChunk {
        id: String,
        data_b64: String,
    },

    DropEnd {
        id: String,
    },

    Ping,
    Pong,
}