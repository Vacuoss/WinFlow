use crate::config::Config;
use crate::cursor_switch::lock_point;
use crate::input_capture::screen_size;
use crate::network::send_udp_message;
use crate::protocol::{Message, MouseButton};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, MSLLHOOKSTRUCT, MSG,
    SetCursorPos, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

struct HookState {
    cfg: Config,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
    suppress_next_move: bool,
}

static HOOK_STATE: OnceLock<Mutex<HookState>> = OnceLock::new();

pub fn start_mouse_hook(
    cfg: Config,
    connected: Arc<AtomicBool>,
    remote_mode: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let _ = HOOK_STATE.set(Mutex::new(HookState {
            cfg,
            connected,
            remote_mode,
            suppress_next_move: false,
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
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let event = w_param.0 as u32;
    let info = *(l_param.0 as *const MSLLHOOKSTRUCT);

    let (w, h) = screen_size();
    let (lock_x, lock_y) = lock_point(&state.cfg, w, h);

    match event {
        WM_MOUSEMOVE => {
            let dx = info.pt.x - lock_x;
            let dy = info.pt.y - lock_y;

            if state.suppress_next_move {
                state.suppress_next_move = false;
                return LRESULT(1);
            }

            if dx.abs() > 1 || dy.abs() > 1 {
                send_udp_message(&state.cfg, &Message::MouseMove { dx, dy });

                let _ = SetCursorPos(lock_x, lock_y);
                state.suppress_next_move = true;
            }

            LRESULT(1)
        }

        WM_LBUTTONDOWN => {
            send_udp_message(
                &state.cfg,
                &Message::MouseButton {
                    button: MouseButton::Left,
                    down: true,
                },
            );

            LRESULT(1)
        }

        WM_LBUTTONUP => {
            send_udp_message(
                &state.cfg,
                &Message::MouseButton {
                    button: MouseButton::Left,
                    down: false,
                },
            );

            LRESULT(1)
        }

        WM_RBUTTONDOWN => {
            send_udp_message(
                &state.cfg,
                &Message::MouseButton {
                    button: MouseButton::Right,
                    down: true,
                },
            );

            LRESULT(1)
        }

        WM_RBUTTONUP => {
            send_udp_message(
                &state.cfg,
                &Message::MouseButton {
                    button: MouseButton::Right,
                    down: false,
                },
            );

            LRESULT(1)
        }

        WM_MBUTTONDOWN => {
            send_udp_message(
                &state.cfg,
                &Message::MouseButton {
                    button: MouseButton::Middle,
                    down: true,
                },
            );

            LRESULT(1)
        }

        WM_MBUTTONUP => {
            send_udp_message(
                &state.cfg,
                &Message::MouseButton {
                    button: MouseButton::Middle,
                    down: false,
                },
            );

            LRESULT(1)
        }

        WM_MOUSEWHEEL => {
            let delta = ((info.mouseData >> 16) & 0xffff) as i16 as i32;

            send_udp_message(&state.cfg, &Message::MouseWheel { delta });

            LRESULT(1)
        }

        _ => CallNextHookEx(None, n_code, w_param, l_param),
    }
}