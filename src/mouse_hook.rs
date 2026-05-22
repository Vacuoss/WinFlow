use crate::config::Config;
use crate::cursor_switch::lock_point;
use crate::input_capture::screen_size;
use crate::network::send_udp_message;
use crate::protocol::{Message, MouseButton};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, ClipCursor, DispatchMessageW, GetMessageW, MSLLHOOKSTRUCT,
    MSG, SetCursorPos, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

const MAX_MOUSE_DELTA: i32 = 160;
const DEADZONE: i32 = 1;
const CLIP_SIZE: i32 = 2;

struct HookState {
    cfg_runtime: Arc<Mutex<Config>>,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
    suppress_next_move: bool,
    cursor_clipped: bool,
}

static HOOK_STATE: OnceLock<Mutex<HookState>> = OnceLock::new();

pub fn start_mouse_hook(
    cfg_runtime: Arc<Mutex<Config>>,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let _ = HOOK_STATE.set(Mutex::new(HookState {
            cfg_runtime,
            connected,
            remote_mode,
            suppress_next_move: false,
            cursor_clipped: false,
        }));

        unsafe {
            let module = GetModuleHandleW(None).unwrap_or_default();

            let hook = match SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_proc),
                Some(HINSTANCE(module.0)),
                0,
            ) {
                Ok(hook) => hook,
                Err(e) => {
                    eprintln!("[mouse_hook] failed to install hook: {e}");
                    return;
                }
            };

            let mut msg = MSG::default();

            while GetMessageW(&mut msg, None, 0, 0).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            release_cursor_clip();

            let _ = UnhookWindowsHookEx(hook);
        }
    });
}

unsafe extern "system" fn mouse_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let Some(state_mutex) = HOOK_STATE.get() else {
        return CallNextHookEx(None, n_code, w_param, l_param);
    };

    let mut state = match state_mutex.lock() {
        Ok(state) => state,
        Err(_) => return CallNextHookEx(None, n_code, w_param, l_param),
    };

    if !state.connected.load(Ordering::Relaxed)
        || !state.remote_mode.load(Ordering::Relaxed)
    {
        reset_remote_state(&mut state);
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let cfg_runtime = state.cfg_runtime.clone();

    let cfg = match cfg_runtime.lock() {
        Ok(cfg) => cfg.clone(),
        Err(_) => {
            reset_remote_state(&mut state);
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
    };

    let event = w_param.0 as u32;
    let info = *(l_param.0 as *const MSLLHOOKSTRUCT);

    let (w, h) = screen_size();
    let (lock_x, lock_y) = lock_point(&cfg, w, h);

    if !state.cursor_clipped {
        let _ = SetCursorPos(lock_x, lock_y);
        clip_cursor_to_point(lock_x, lock_y);

        state.cursor_clipped = true;
        state.suppress_next_move = true;
    }

    match event {
        WM_MOUSEMOVE => {
            let dx = (info.pt.x - lock_x).clamp(-MAX_MOUSE_DELTA, MAX_MOUSE_DELTA);
            let dy = (info.pt.y - lock_y).clamp(-MAX_MOUSE_DELTA, MAX_MOUSE_DELTA);

            if state.suppress_next_move {
                state.suppress_next_move = false;
                return LRESULT(1);
            }

            if dx.abs() <= DEADZONE && dy.abs() <= DEADZONE {
                return LRESULT(1);
            }

            send_udp_message(&cfg, &Message::MouseMove { dx, dy });

            let _ = SetCursorPos(lock_x, lock_y);
            clip_cursor_to_point(lock_x, lock_y);

            state.suppress_next_move = true;

            LRESULT(1)
        }

        WM_LBUTTONDOWN => {
            send_mouse_button(&cfg, MouseButton::Left, true);
            LRESULT(1)
        }

        WM_LBUTTONUP => {
            send_mouse_button(&cfg, MouseButton::Left, false);
            LRESULT(1)
        }

        WM_RBUTTONDOWN => {
            send_mouse_button(&cfg, MouseButton::Right, true);
            LRESULT(1)
        }

        WM_RBUTTONUP => {
            send_mouse_button(&cfg, MouseButton::Right, false);
            LRESULT(1)
        }

        WM_MBUTTONDOWN => {
            send_mouse_button(&cfg, MouseButton::Middle, true);
            LRESULT(1)
        }

        WM_MBUTTONUP => {
            send_mouse_button(&cfg, MouseButton::Middle, false);
            LRESULT(1)
        }

        WM_MOUSEWHEEL => {
            let delta = ((info.mouseData >> 16) & 0xffff) as i16 as i32;

            send_udp_message(&cfg, &Message::MouseWheel { delta });

            LRESULT(1)
        }

        _ => CallNextHookEx(None, n_code, w_param, l_param),
    }
}

fn reset_remote_state(state: &mut HookState) {
    state.suppress_next_move = false;

    if state.cursor_clipped {
        release_cursor_clip();
        state.cursor_clipped = false;
    }
}

fn send_mouse_button(cfg: &Config, button: MouseButton, down: bool) {
    send_udp_message(
        cfg,
        &Message::MouseButton {
            button,
            down,
        },
    );
}

fn clip_cursor_to_point(x: i32, y: i32) {
    let rect = RECT {
        left: x - CLIP_SIZE,
        top: y - CLIP_SIZE,
        right: x + CLIP_SIZE,
        bottom: y + CLIP_SIZE,
    };

    unsafe {
        let _ = ClipCursor(Some(&rect));
    }
}

fn release_cursor_clip() {
    unsafe {
        let _ = ClipCursor(None);
    }
}