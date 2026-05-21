use crate::config::Config;
use crate::cursor_switch::lock_point;
use crate::network::send_udp_message;
use crate::protocol::{Message, MouseButton};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::POINT;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState,
    VK_LBUTTON,
    VK_MBUTTON,
    VK_RBUTTON,
};

use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos,
    GetSystemMetrics,
    SetCursorPos,
    SM_CXSCREEN,
    SM_CYSCREEN,
};

pub fn start_input_capture(
    cfg: Config,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut last_left = false;
        let mut last_right = false;
        let mut last_middle = false;
        let mut last_send = Instant::now();
        let mut ignore_next_warp = false;
        let mut was_remote = false;
        let tick = Duration::from_millis(8);

        loop {
            if !connected.load(Ordering::Relaxed) {
                reset_mouse_state(
                    &mut was_remote,
                    &mut ignore_next_warp,
                    &mut last_left,
                    &mut last_right,
                    &mut last_middle,
                );

                thread::sleep(Duration::from_millis(100));
                continue;
            }

            if !remote_mode.load(Ordering::Relaxed) {
                reset_mouse_state(
                    &mut was_remote,
                    &mut ignore_next_warp,
                    &mut last_left,
                    &mut last_right,
                    &mut last_middle,
                );

                thread::sleep(tick);
                continue;
            }

            let (w, h) = screen_size();
            let (lock_x, lock_y) = lock_point(&cfg, w, h);

            if !was_remote {
                unsafe {
                    let _ = SetCursorPos(lock_x, lock_y);
                }

                was_remote = true;
                ignore_next_warp = true;

                thread::sleep(Duration::from_millis(24));
                continue;
            }

            let mut p = POINT::default();

            unsafe {
                if GetCursorPos(&mut p).is_err() {
                    thread::sleep(tick);
                    continue;
                }
            }

            let dx = p.x - lock_x;
            let dy = p.y - lock_y;
            let left = key_down(VK_LBUTTON.0 as i32);
            let right = key_down(VK_RBUTTON.0 as i32);
            let middle = key_down(VK_MBUTTON.0 as i32);

            send_button_if_changed(&cfg, MouseButton::Left, left, &mut last_left);
            send_button_if_changed(&cfg, MouseButton::Right, right, &mut last_right);
            send_button_if_changed(&cfg, MouseButton::Middle, middle, &mut last_middle);

            if ignore_next_warp {
                ignore_next_warp = false;

                thread::sleep(tick);
                continue;
            }
            if dx.abs() <= 1 && dy.abs() <= 1 {
                thread::sleep(tick);
                continue;
            }

            if last_send.elapsed() >= tick {
                send_udp_message(&cfg, &Message::MouseMove { dx, dy });
                last_send = Instant::now();

                unsafe {
                    let _ = SetCursorPos(lock_x, lock_y);
                }

                ignore_next_warp = true;
            }

            thread::sleep(tick);
        }
    });
}

fn reset_mouse_state(
    was_remote: &mut bool,
    ignore_next_warp: &mut bool,
    last_left: &mut bool,
    last_right: &mut bool,
    last_middle: &mut bool,
) {
    *was_remote = false;
    *ignore_next_warp = false;

    *last_left = false;
    *last_right = false;
    *last_middle = false;
}

fn send_button_if_changed(
    cfg: &Config,
    button: MouseButton,
    current: bool,
    previous: &mut bool,
) {
    if current != *previous {
        send_udp_message(
            cfg,
            &Message::MouseButton {
                button,
                down: current,
            },
        );

        *previous = current;
    }
}

fn key_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

pub fn screen_size() -> (i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
        )
    }
}