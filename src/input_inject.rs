use crate::protocol::MouseButton;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos,
    GetSystemMetrics,
    SetCursorPos,
    SM_CXSCREEN,
    SM_CYSCREEN,
};

pub fn inject_mouse_move(dx: i32, dy: i32) {
    unsafe {
        let mut p = POINT::default();

        if GetCursorPos(&mut p).is_ok() {
            let _ = SetCursorPos(p.x + dx, p.y + dy);
        }
    }
}

pub fn inject_mouse_wheel(delta: i32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn place_cursor_from_edge(edge: String) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);

        match edge.as_str() {
            "right" => {
                let _ = SetCursorPos(40, h / 2);
            }
            "left" => {
                let _ = SetCursorPos(w - 40, h / 2);
            }
            "top" => {
                let _ = SetCursorPos(w / 2, h - 40);
            }
            "bottom" => {
                let _ = SetCursorPos(w / 2, 40);
            }
            _ => {
                let _ = SetCursorPos(w / 2, h / 2);
            }
        }
    }
}

pub fn inject_mouse_button(button: MouseButton, down: bool) {
    let flags = match (button, down) {
        (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
        (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
        (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
        (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
    };

    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}