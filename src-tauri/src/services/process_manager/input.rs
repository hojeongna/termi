use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use super::types::{VK_W, VK_F24};

/// SendInput으로 Ctrl+Shift+W 키 입력을 전송한다.
pub(super) fn send_ctrl_shift_w() {
    // SAFETY: INPUT is a C FFI struct where all-zeros is a valid representation.
    let mut inputs: [INPUT; 6] = unsafe { std::mem::zeroed() };

    // Ctrl down
    inputs[0].r#type = INPUT_KEYBOARD;
    inputs[0].Anonymous.ki.wVk = VK_CONTROL;

    // Shift down
    inputs[1].r#type = INPUT_KEYBOARD;
    inputs[1].Anonymous.ki.wVk = VK_SHIFT;

    // W down
    inputs[2].r#type = INPUT_KEYBOARD;
    inputs[2].Anonymous.ki.wVk = VK_W;

    // W up
    inputs[3].r#type = INPUT_KEYBOARD;
    inputs[3].Anonymous.ki.wVk = VK_W;
    inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // Shift up
    inputs[4].r#type = INPUT_KEYBOARD;
    inputs[4].Anonymous.ki.wVk = VK_SHIFT;
    inputs[4].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // Ctrl up
    inputs[5].r#type = INPUT_KEYBOARD;
    inputs[5].Anonymous.ki.wVk = VK_CONTROL;
    inputs[5].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // SAFETY: inputs array is correctly sized and initialized; size_of::<INPUT>() matches the expected cbSize.
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}

/// SendInput으로 Ctrl+Shift+Alt+F24 키 입력을 전송한다.
/// WT의 moveTab 키바인딩을 발동시키기 위한 함수.
pub(super) fn send_ctrl_shift_alt_f24() {
    // SAFETY: INPUT is a C FFI struct where all-zeros is a valid representation.
    let mut inputs: [INPUT; 8] = unsafe { std::mem::zeroed() };

    // Ctrl down
    inputs[0].r#type = INPUT_KEYBOARD;
    inputs[0].Anonymous.ki.wVk = VK_CONTROL;

    // Shift down
    inputs[1].r#type = INPUT_KEYBOARD;
    inputs[1].Anonymous.ki.wVk = VK_SHIFT;

    // Alt down
    inputs[2].r#type = INPUT_KEYBOARD;
    inputs[2].Anonymous.ki.wVk = VK_MENU;

    // F24 down
    inputs[3].r#type = INPUT_KEYBOARD;
    inputs[3].Anonymous.ki.wVk = VK_F24;

    // F24 up
    inputs[4].r#type = INPUT_KEYBOARD;
    inputs[4].Anonymous.ki.wVk = VK_F24;
    inputs[4].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // Alt up
    inputs[5].r#type = INPUT_KEYBOARD;
    inputs[5].Anonymous.ki.wVk = VK_MENU;
    inputs[5].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // Shift up
    inputs[6].r#type = INPUT_KEYBOARD;
    inputs[6].Anonymous.ki.wVk = VK_SHIFT;
    inputs[6].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // Ctrl up
    inputs[7].r#type = INPUT_KEYBOARD;
    inputs[7].Anonymous.ki.wVk = VK_CONTROL;
    inputs[7].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

    // SAFETY: inputs array is correctly sized and initialized; size_of::<INPUT>() matches the expected cbSize.
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}
