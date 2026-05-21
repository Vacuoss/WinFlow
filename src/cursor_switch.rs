use crate::config::Config;
use crate::input_capture::screen_size;
use crate::network::send_udp_message;
use crate::protocol::Message;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};
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
pub fn start_cursor_switch(
    cfg: Config,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut last_hotkey = Instant::now();

        loop {
            if !connected.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let mut p = POINT::default();
            let (w, h) = screen_size();

            unsafe {
                if GetCursorPos(&mut p).is_ok() {
                    let already_remote = remote_mode.load(Ordering::Relaxed);

                    if hotkey_pressed(&cfg.hotkey_disconnect)
                        && last_hotkey.elapsed() > Duration::from_millis(500)
                    {
                        remote_mode.store(false, Ordering::Relaxed);
                        send_udp_message(&cfg, &Message::ReturnControl);
                        move_to_safe_point(&cfg, w, h);

                        last_hotkey = Instant::now();
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }

                    let edge_trigger = match cfg.switch_edge.as_str() {
                        "right" => p.x >= w - 2,
                        "left" => p.x <= 1,
                        "top" => p.y <= 1,
                        "bottom" => p.y >= h - 2,
                        _ => false,
                    };

                    if edge_trigger && !already_remote {
                        remote_mode.store(true, Ordering::Relaxed);

                        send_udp_message(
                            &cfg,
                            &Message::EnterControl {
                                edge: cfg.switch_edge.clone(),
                            },
                        );

                        move_to_lock_point(&cfg, w, h);
                        thread::sleep(Duration::from_millis(250));
                    }

                    if hotkey_pressed(&cfg.hotkey_switch)
                        && last_hotkey.elapsed() > Duration::from_millis(500)
                    {
                        if already_remote {
                            remote_mode.store(false, Ordering::Relaxed);
                            send_udp_message(&cfg, &Message::ReturnControl);
                            move_to_safe_point(&cfg, w, h);
                        } else {
                            remote_mode.store(true, Ordering::Relaxed);

                            send_udp_message(
                                &cfg,
                                &Message::EnterControl {
                                    edge: cfg.switch_edge.clone(),
                                },
                            );

                            move_to_lock_point(&cfg, w, h);
                        }

                        last_hotkey = Instant::now();
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            thread::sleep(Duration::from_millis(16));
        }
    });
}

pub fn lock_point(cfg: &Config, w: i32, h: i32) -> (i32, i32) {
    match cfg.switch_edge.as_str() {
        "right" => (w - 80, h / 2),
        "left" => (80, h / 2),
        "top" => (w / 2, 80),
        "bottom" => (w / 2, h - 80),
        _ => (w / 2, h / 2),
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
                let _ = SetCursorPos(w - 160, h / 2);
            }
            "left" => {
                let _ = SetCursorPos(160, h / 2);
            }
            "top" => {
                let _ = SetCursorPos(w / 2, 160);
            }
            "bottom" => {
                let _ = SetCursorPos(w / 2, h - 160);
            }
            _ => {
                let _ = SetCursorPos(w / 2, h / 2);
            }
        }
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
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}