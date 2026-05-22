use crate::config::Config;
use crate::input_capture::screen_size;
use crate::network::send_udp_message;
use crate::protocol::Message;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{ClipCursor, GetCursorPos, SetCursorPos};

const VK_CONTROL: i32 = 0x11;
const VK_SHIFT: i32 = 0x10;
const VK_MENU: i32 = 0x12;

const VK_LMENU: i32 = 0xA4;
const VK_RMENU: i32 = 0xA5;

const VK_LEFT: i32 = 0x25;
const VK_UP: i32 = 0x26;
const VK_RIGHT: i32 = 0x27;
const VK_DOWN: i32 = 0x28;

const VK_A: i32 = 0x41;
const VK_B: i32 = 0x42;
const VK_C: i32 = 0x43;
const VK_D: i32 = 0x44;
const VK_E: i32 = 0x45;
const VK_F: i32 = 0x46;
const VK_G: i32 = 0x47;
const VK_H: i32 = 0x48;
const VK_I: i32 = 0x49;
const VK_J: i32 = 0x4A;
const VK_K: i32 = 0x4B;
const VK_L: i32 = 0x4C;
const VK_M: i32 = 0x4D;
const VK_N: i32 = 0x4E;
const VK_O: i32 = 0x4F;
const VK_P: i32 = 0x50;
const VK_Q: i32 = 0x51;
const VK_R: i32 = 0x52;
const VK_S: i32 = 0x53;
const VK_T: i32 = 0x54;
const VK_U: i32 = 0x55;
const VK_V: i32 = 0x56;
const VK_W: i32 = 0x57;
const VK_X: i32 = 0x58;
const VK_Y: i32 = 0x59;
const VK_Z: i32 = 0x5A;

const HORIZONTAL_EDGE: i32 = 10;
const TOP_EDGE: i32 = 6;
const BOTTOM_EDGE: i32 = 14;

const INITIAL_PUSH: i32 = 36;
const SWITCH_COOLDOWN_MS: u64 = 260;

pub fn start_cursor_switch(
    cfg_runtime: Arc<Mutex<Config>>,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut last_switch = Instant::now();

        loop {
            if !connected.load(Ordering::Relaxed) {
                release_cursor_clip();
                thread::sleep(Duration::from_millis(80));
                continue;
            }

            let cfg = match cfg_runtime.lock() {
                Ok(cfg) => cfg.clone(),
                Err(_) => {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };

            let (w, h) = screen_size();
            let already_remote = remote_mode.load(Ordering::Relaxed);

            if hotkey_pressed(&cfg.hotkey_disconnect)
                && last_switch.elapsed() > Duration::from_millis(SWITCH_COOLDOWN_MS)
            {
                remote_mode.store(false, Ordering::Relaxed);
                release_cursor_clip();

                send_udp_message(&cfg, &Message::ReturnControl);
                move_to_safe_point(&cfg, w, h);

                last_switch = Instant::now();
                thread::sleep(Duration::from_millis(70));
                continue;
            }

            if hotkey_pressed(&cfg.hotkey_switch)
                && last_switch.elapsed() > Duration::from_millis(SWITCH_COOLDOWN_MS)
            {
                if already_remote {
                    remote_mode.store(false, Ordering::Relaxed);
                    release_cursor_clip();

                    send_udp_message(&cfg, &Message::ReturnControl);
                    move_to_safe_point(&cfg, w, h);
                } else {
                    enter_remote_mode(&cfg, &remote_mode, w, h);
                }

                last_switch = Instant::now();
                thread::sleep(Duration::from_millis(70));
                continue;
            }

            if !already_remote
                && last_switch.elapsed() > Duration::from_millis(SWITCH_COOLDOWN_MS)
            {
                let mut p = POINT::default();

                unsafe {
                    if GetCursorPos(&mut p).is_ok() && edge_triggered(&cfg, p.x, p.y, w, h) {
                        enter_remote_mode(&cfg, &remote_mode, w, h);

                        last_switch = Instant::now();
                        thread::sleep(Duration::from_millis(70));
                        continue;
                    }
                }
            }

            thread::sleep(Duration::from_millis(6));
        }
    });
}

pub fn lock_point(cfg: &Config, w: i32, h: i32) -> (i32, i32) {
    match cfg.switch_edge.as_str() {
        "right" => (w - 80, h / 2),
        "left" => (80, h / 2),
        "top" => (w / 2, 32),
        "bottom" => (w / 2, h - 32),
        _ => (w / 2, h / 2),
    }
}

fn enter_remote_mode(
    cfg: &Config,
    remote_mode: &Arc<AtomicBool>,
    w: i32,
    h: i32,
) {
    remote_mode.store(true, Ordering::Relaxed);

    send_udp_message(
        cfg,
        &Message::EnterControl {
            edge: cfg.switch_edge.clone(),
        },
    );

    move_to_lock_point(cfg, w, h);

    send_initial_push(cfg);
}

fn edge_triggered(cfg: &Config, x: i32, y: i32, w: i32, h: i32) -> bool {
    match cfg.switch_edge.as_str() {
        "right" => x >= w - HORIZONTAL_EDGE,
        "left" => x <= HORIZONTAL_EDGE,
        "top" => y <= TOP_EDGE,
        "bottom" => y >= h - BOTTOM_EDGE,
        _ => false,
    }
}

fn send_initial_push(cfg: &Config) {
    let (dx, dy) = match cfg.switch_edge.as_str() {
        "right" => (INITIAL_PUSH, 0),
        "left" => (-INITIAL_PUSH, 0),
        "top" => (0, -INITIAL_PUSH),
        "bottom" => (0, INITIAL_PUSH),
        _ => (0, 0),
    };

    if dx != 0 || dy != 0 {
        send_udp_message(cfg, &Message::MouseMove { dx, dy });
    }
}

fn move_to_lock_point(cfg: &Config, w: i32, h: i32) {
    let (x, y) = lock_point(cfg, w, h);

    unsafe {
        let _ = SetCursorPos(x, y);
    }
}

fn move_to_safe_point(cfg: &Config, w: i32, h: i32) {
    unsafe {
        match cfg.switch_edge.as_str() {
            "right" => {
                let _ = SetCursorPos(w - 220, h / 2);
            }
            "left" => {
                let _ = SetCursorPos(220, h / 2);
            }
            "top" => {
                let _ = SetCursorPos(w / 2, 180);
            }
            "bottom" => {
                let _ = SetCursorPos(w / 2, h - 180);
            }
            _ => {
                let _ = SetCursorPos(w / 2, h / 2);
            }
        }
    }
}

fn release_cursor_clip() {
    unsafe {
        let _ = ClipCursor(None);
    }
}

fn hotkey_pressed(raw: &str) -> bool {
    let parts: Vec<String> = raw
        .split('+')
        .map(|p| p.trim().to_lowercase().replace(' ', ""))
        .filter(|p| !p.is_empty())
        .collect();

    if parts.is_empty() {
        return false;
    }

    for part in parts {
        match part.as_str() {
            "ctrl" | "control" => {
                if !key_down(VK_CONTROL) {
                    return false;
                }
            }
            "shift" => {
                if !key_down(VK_SHIFT) {
                    return false;
                }
            }
            "alt" => {
                if !key_down(VK_MENU) {
                    return false;
                }
            }
            "leftalt" | "lalt" => {
                if !key_down(VK_LMENU) {
                    return false;
                }
            }
            "rightalt" | "ralt" => {
                if !key_down(VK_RMENU) {
                    return false;
                }
            }
            "left" => {
                if !key_down(VK_LEFT) {
                    return false;
                }
            }
            "right" => {
                if !key_down(VK_RIGHT) {
                    return false;
                }
            }
            "up" => {
                if !key_down(VK_UP) {
                    return false;
                }
            }
            "down" => {
                if !key_down(VK_DOWN) {
                    return false;
                }
            }

            "a" => if !key_down(VK_A) { return false; },
            "b" => if !key_down(VK_B) { return false; },
            "c" => if !key_down(VK_C) { return false; },
            "d" => if !key_down(VK_D) { return false; },
            "e" => if !key_down(VK_E) { return false; },
            "f" => if !key_down(VK_F) { return false; },
            "g" => if !key_down(VK_G) { return false; },
            "h" => if !key_down(VK_H) { return false; },
            "i" => if !key_down(VK_I) { return false; },
            "j" => if !key_down(VK_J) { return false; },
            "k" => if !key_down(VK_K) { return false; },
            "l" => if !key_down(VK_L) { return false; },
            "m" => if !key_down(VK_M) { return false; },
            "n" => if !key_down(VK_N) { return false; },
            "o" => if !key_down(VK_O) { return false; },
            "p" => if !key_down(VK_P) { return false; },
            "q" => if !key_down(VK_Q) { return false; },
            "r" => if !key_down(VK_R) { return false; },
            "s" => if !key_down(VK_S) { return false; },
            "t" => if !key_down(VK_T) { return false; },
            "u" => if !key_down(VK_U) { return false; },
            "v" => if !key_down(VK_V) { return false; },
            "w" => if !key_down(VK_W) { return false; },
            "x" => if !key_down(VK_X) { return false; },
            "y" => if !key_down(VK_Y) { return false; },
            "z" => if !key_down(VK_Z) { return false; },

            _ => return false,
        }
    }

    true
}

fn key_down(vk: i32) -> bool {
    unsafe {
        (GetAsyncKeyState(vk) as u16 & 0x8000) != 0
    }
}