use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigUpdate {
    pub switch_edge: String,
    pub clipboard_enabled: bool,
    pub hotkey_switch: String,
    pub hotkey_disconnect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
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

    Ping,
    Pong,
    Disconnect,

    ConfigUpdate(RuntimeConfigUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub token: String,
    pub message: Message,
}