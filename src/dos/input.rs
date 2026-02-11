use std::collections::HashMap;

pub use raylib::ffi::KeyboardKey;
use raylib::{RaylibHandle, ffi::MouseButton};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InputState {
    pub down: bool,
    pub pressed: bool,
    pub released: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Input {
    pub codes: HashMap<i32, InputState>,
    pub mouse: HashMap<i32, InputState>,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub scroll_amount: f32,
}

macro_rules! handle_key {
    ($handle:ident, $value:ident, $out:ident) => {{
        $out.insert(
            KeyboardKey::$value as i32,
            InputState {
                down: $handle.is_key_down(KeyboardKey::$value),
                pressed: $handle.is_key_pressed(KeyboardKey::$value),
                released: $handle.is_key_released(KeyboardKey::$value),
            },
        )
    }};
}

macro_rules! handle_mouse {
    ($handle:ident, $value:ident, $out:ident) => {{
        $out.insert(
            MouseButton::$value as i32,
            InputState {
                down: $handle.is_mouse_button_down(MouseButton::$value),
                pressed: $handle.is_mouse_button_pressed(MouseButton::$value),
                released: $handle.is_mouse_button_released(MouseButton::$value),
            },
        )
    }};
}
macro_rules! handle_key_empty {
    ($_handle:ident, $value:ident, $out:ident) => {{
        $out.insert(
            KeyboardKey::$value as i32,
            InputState {
                down: false,
                pressed: false,
                released: false,
            },
        )
    }};
}

macro_rules! handle_mouse_empty {
    ($_handle:ident, $value:ident, $out:ident) => {{
        $out.insert(
            MouseButton::$value as i32,
            InputState {
                down: false,
                pressed: false,
                released: false,
            },
        )
    }};
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        let mut out = HashMap::new();
        out.reserve(107);
        let _handle = ();
        handle_key_empty!(handle, KEY_NULL, out);
        handle_key_empty!(handle, KEY_APOSTROPHE, out);
        handle_key_empty!(handle, KEY_COMMA, out);
        handle_key_empty!(handle, KEY_MINUS, out);
        handle_key_empty!(handle, KEY_PERIOD, out);
        handle_key_empty!(handle, KEY_SLASH, out);
        handle_key_empty!(handle, KEY_ONE, out);
        handle_key_empty!(handle, KEY_TWO, out);
        handle_key_empty!(handle, KEY_THREE, out);
        handle_key_empty!(handle, KEY_FOUR, out);
        handle_key_empty!(handle, KEY_FIVE, out);
        handle_key_empty!(handle, KEY_SIX, out);
        handle_key_empty!(handle, KEY_SEVEN, out);
        handle_key_empty!(handle, KEY_EIGHT, out);
        handle_key_empty!(handle, KEY_NINE, out);
        handle_key_empty!(handle, KEY_SEMICOLON, out);
        handle_key_empty!(handle, KEY_EQUAL, out);
        handle_key_empty!(handle, KEY_A, out);
        handle_key_empty!(handle, KEY_B, out);
        handle_key_empty!(handle, KEY_C, out);
        handle_key_empty!(handle, KEY_D, out);
        handle_key_empty!(handle, KEY_E, out);
        handle_key_empty!(handle, KEY_F, out);
        handle_key_empty!(handle, KEY_G, out);
        handle_key_empty!(handle, KEY_H, out);
        handle_key_empty!(handle, KEY_I, out);
        handle_key_empty!(handle, KEY_J, out);
        handle_key_empty!(handle, KEY_K, out);
        handle_key_empty!(handle, KEY_L, out);
        handle_key_empty!(handle, KEY_M, out);
        handle_key_empty!(handle, KEY_N, out);
        handle_key_empty!(handle, KEY_O, out);
        handle_key_empty!(handle, KEY_P, out);
        handle_key_empty!(handle, KEY_Q, out);
        handle_key_empty!(handle, KEY_R, out);
        handle_key_empty!(handle, KEY_S, out);
        handle_key_empty!(handle, KEY_T, out);
        handle_key_empty!(handle, KEY_U, out);
        handle_key_empty!(handle, KEY_V, out);
        handle_key_empty!(handle, KEY_W, out);
        handle_key_empty!(handle, KEY_X, out);
        handle_key_empty!(handle, KEY_Y, out);
        handle_key_empty!(handle, KEY_Z, out);
        handle_key_empty!(handle, KEY_LEFT_BRACKET, out);
        handle_key_empty!(handle, KEY_BACKSLASH, out);
        handle_key_empty!(handle, KEY_RIGHT_BRACKET, out);
        handle_key_empty!(handle, KEY_GRAVE, out);
        handle_key_empty!(handle, KEY_SPACE, out);
        handle_key_empty!(handle, KEY_ESCAPE, out);
        handle_key_empty!(handle, KEY_ENTER, out);
        handle_key_empty!(handle, KEY_TAB, out);
        handle_key_empty!(handle, KEY_BACKSPACE, out);
        handle_key_empty!(handle, KEY_INSERT, out);
        handle_key_empty!(handle, KEY_DELETE, out);
        handle_key_empty!(handle, KEY_RIGHT, out);
        handle_key_empty!(handle, KEY_LEFT, out);
        handle_key_empty!(handle, KEY_DOWN, out);
        handle_key_empty!(handle, KEY_UP, out);
        handle_key_empty!(handle, KEY_PAGE_UP, out);
        handle_key_empty!(handle, KEY_PAGE_DOWN, out);
        handle_key_empty!(handle, KEY_HOME, out);
        handle_key_empty!(handle, KEY_END, out);
        handle_key_empty!(handle, KEY_CAPS_LOCK, out);
        handle_key_empty!(handle, KEY_NUM_LOCK, out);
        handle_key_empty!(handle, KEY_PRINT_SCREEN, out);
        handle_key_empty!(handle, KEY_PAUSE, out);
        handle_key_empty!(handle, KEY_F1, out);
        handle_key_empty!(handle, KEY_F2, out);
        handle_key_empty!(handle, KEY_F3, out);
        handle_key_empty!(handle, KEY_F4, out);
        handle_key_empty!(handle, KEY_F5, out);
        handle_key_empty!(handle, KEY_F6, out);
        handle_key_empty!(handle, KEY_F7, out);
        handle_key_empty!(handle, KEY_F8, out);
        handle_key_empty!(handle, KEY_F9, out);
        handle_key_empty!(handle, KEY_F10, out);
        handle_key_empty!(handle, KEY_F11, out);
        handle_key_empty!(handle, KEY_F12, out);
        handle_key_empty!(handle, KEY_LEFT_SHIFT, out);
        handle_key_empty!(handle, KEY_LEFT_CONTROL, out);
        handle_key_empty!(handle, KEY_LEFT_ALT, out);
        handle_key_empty!(handle, KEY_LEFT_SUPER, out);
        handle_key_empty!(handle, KEY_RIGHT_SHIFT, out);
        handle_key_empty!(handle, KEY_RIGHT_CONTROL, out);
        handle_key_empty!(handle, KEY_RIGHT_ALT, out);
        handle_key_empty!(handle, KEY_RIGHT_SUPER, out);
        handle_key_empty!(handle, KEY_KB_MENU, out);
        handle_key_empty!(handle, KEY_KP_0, out);
        handle_key_empty!(handle, KEY_KP_1, out);
        handle_key_empty!(handle, KEY_KP_2, out);
        handle_key_empty!(handle, KEY_KP_3, out);
        handle_key_empty!(handle, KEY_KP_4, out);
        handle_key_empty!(handle, KEY_KP_5, out);
        handle_key_empty!(handle, KEY_KP_6, out);
        handle_key_empty!(handle, KEY_KP_7, out);
        handle_key_empty!(handle, KEY_KP_8, out);
        handle_key_empty!(handle, KEY_KP_9, out);
        handle_key_empty!(handle, KEY_KP_DECIMAL, out);
        handle_key_empty!(handle, KEY_KP_DIVIDE, out);
        handle_key_empty!(handle, KEY_KP_MULTIPLY, out);
        handle_key_empty!(handle, KEY_KP_SUBTRACT, out);
        handle_key_empty!(handle, KEY_KP_ADD, out);
        handle_key_empty!(handle, KEY_KP_ENTER, out);
        handle_key_empty!(handle, KEY_KP_EQUAL, out);
        handle_key_empty!(handle, KEY_BACK, out);
        handle_key_empty!(handle, KEY_MENU, out);
        handle_key_empty!(handle, KEY_VOLUME_UP, out);
        handle_key_empty!(handle, KEY_VOLUME_DOWN, out);
        let mut out2 = HashMap::new();
        handle_mouse_empty!(handle, MOUSE_BUTTON_LEFT, out2);
        handle_mouse_empty!(handle, MOUSE_BUTTON_RIGHT, out2);
        handle_mouse_empty!(handle, MOUSE_BUTTON_MIDDLE, out2);
        handle_mouse_empty!(handle, MOUSE_BUTTON_SIDE, out2);
        handle_mouse_empty!(handle, MOUSE_BUTTON_EXTRA, out2);
        handle_mouse_empty!(handle, MOUSE_BUTTON_FORWARD, out2);
        handle_mouse_empty!(handle, MOUSE_BUTTON_BACK, out2);
        Self {
            codes: out,
            mouse: out2,
            scroll_amount: 0.0,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            mouse_x: 0,
            mouse_y: 0,
        }
    }
}
pub fn generate_input(handle: &mut RaylibHandle) -> Input {
    let mut out = HashMap::new();
    out.reserve(107);
    handle_key!(handle, KEY_NULL, out);
    handle_key!(handle, KEY_APOSTROPHE, out);
    handle_key!(handle, KEY_COMMA, out);
    handle_key!(handle, KEY_MINUS, out);
    handle_key!(handle, KEY_PERIOD, out);
    handle_key!(handle, KEY_SLASH, out);
    handle_key!(handle, KEY_ONE, out);
    handle_key!(handle, KEY_TWO, out);
    handle_key!(handle, KEY_THREE, out);
    handle_key!(handle, KEY_FOUR, out);
    handle_key!(handle, KEY_FIVE, out);
    handle_key!(handle, KEY_SIX, out);
    handle_key!(handle, KEY_SEVEN, out);
    handle_key!(handle, KEY_EIGHT, out);
    handle_key!(handle, KEY_NINE, out);
    handle_key!(handle, KEY_SEMICOLON, out);
    handle_key!(handle, KEY_EQUAL, out);
    handle_key!(handle, KEY_A, out);
    handle_key!(handle, KEY_B, out);
    handle_key!(handle, KEY_C, out);
    handle_key!(handle, KEY_D, out);
    handle_key!(handle, KEY_E, out);
    handle_key!(handle, KEY_F, out);
    handle_key!(handle, KEY_G, out);
    handle_key!(handle, KEY_H, out);
    handle_key!(handle, KEY_I, out);
    handle_key!(handle, KEY_J, out);
    handle_key!(handle, KEY_K, out);
    handle_key!(handle, KEY_L, out);
    handle_key!(handle, KEY_M, out);
    handle_key!(handle, KEY_N, out);
    handle_key!(handle, KEY_O, out);
    handle_key!(handle, KEY_P, out);
    handle_key!(handle, KEY_Q, out);
    handle_key!(handle, KEY_R, out);
    handle_key!(handle, KEY_S, out);
    handle_key!(handle, KEY_T, out);
    handle_key!(handle, KEY_U, out);
    handle_key!(handle, KEY_V, out);
    handle_key!(handle, KEY_W, out);
    handle_key!(handle, KEY_X, out);
    handle_key!(handle, KEY_Y, out);
    handle_key!(handle, KEY_Z, out);
    handle_key!(handle, KEY_LEFT_BRACKET, out);
    handle_key!(handle, KEY_BACKSLASH, out);
    handle_key!(handle, KEY_RIGHT_BRACKET, out);
    handle_key!(handle, KEY_GRAVE, out);
    handle_key!(handle, KEY_SPACE, out);
    handle_key!(handle, KEY_ESCAPE, out);
    handle_key!(handle, KEY_ENTER, out);
    handle_key!(handle, KEY_TAB, out);
    handle_key!(handle, KEY_BACKSPACE, out);
    handle_key!(handle, KEY_INSERT, out);
    handle_key!(handle, KEY_DELETE, out);
    handle_key!(handle, KEY_RIGHT, out);
    handle_key!(handle, KEY_LEFT, out);
    handle_key!(handle, KEY_DOWN, out);
    handle_key!(handle, KEY_UP, out);
    handle_key!(handle, KEY_PAGE_UP, out);
    handle_key!(handle, KEY_PAGE_DOWN, out);
    handle_key!(handle, KEY_HOME, out);
    handle_key!(handle, KEY_END, out);
    handle_key!(handle, KEY_CAPS_LOCK, out);
    handle_key!(handle, KEY_NUM_LOCK, out);
    handle_key!(handle, KEY_PRINT_SCREEN, out);
    handle_key!(handle, KEY_PAUSE, out);
    handle_key!(handle, KEY_F1, out);
    handle_key!(handle, KEY_F2, out);
    handle_key!(handle, KEY_F3, out);
    handle_key!(handle, KEY_F4, out);
    handle_key!(handle, KEY_F5, out);
    handle_key!(handle, KEY_F6, out);
    handle_key!(handle, KEY_F7, out);
    handle_key!(handle, KEY_F8, out);
    handle_key!(handle, KEY_F9, out);
    handle_key!(handle, KEY_F10, out);
    handle_key!(handle, KEY_F11, out);
    handle_key!(handle, KEY_F12, out);
    handle_key!(handle, KEY_LEFT_SHIFT, out);
    handle_key!(handle, KEY_LEFT_CONTROL, out);
    handle_key!(handle, KEY_LEFT_ALT, out);
    handle_key!(handle, KEY_LEFT_SUPER, out);
    handle_key!(handle, KEY_RIGHT_SHIFT, out);
    handle_key!(handle, KEY_RIGHT_CONTROL, out);
    handle_key!(handle, KEY_RIGHT_ALT, out);
    handle_key!(handle, KEY_RIGHT_SUPER, out);
    handle_key!(handle, KEY_KB_MENU, out);
    handle_key!(handle, KEY_KP_0, out);
    handle_key!(handle, KEY_KP_1, out);
    handle_key!(handle, KEY_KP_2, out);
    handle_key!(handle, KEY_KP_3, out);
    handle_key!(handle, KEY_KP_4, out);
    handle_key!(handle, KEY_KP_5, out);
    handle_key!(handle, KEY_KP_6, out);
    handle_key!(handle, KEY_KP_7, out);
    handle_key!(handle, KEY_KP_8, out);
    handle_key!(handle, KEY_KP_9, out);
    handle_key!(handle, KEY_KP_DECIMAL, out);
    handle_key!(handle, KEY_KP_DIVIDE, out);
    handle_key!(handle, KEY_KP_MULTIPLY, out);
    handle_key!(handle, KEY_KP_SUBTRACT, out);
    handle_key!(handle, KEY_KP_ADD, out);
    handle_key!(handle, KEY_KP_ENTER, out);
    handle_key!(handle, KEY_KP_EQUAL, out);
    handle_key!(handle, KEY_BACK, out);
    handle_key!(handle, KEY_MENU, out);
    handle_key!(handle, KEY_VOLUME_UP, out);
    handle_key!(handle, KEY_VOLUME_DOWN, out);
    let mut out2 = HashMap::new();
    handle_mouse!(handle, MOUSE_BUTTON_LEFT, out2);
    handle_mouse!(handle, MOUSE_BUTTON_RIGHT, out2);
    handle_mouse!(handle, MOUSE_BUTTON_MIDDLE, out2);
    handle_mouse!(handle, MOUSE_BUTTON_SIDE, out2);
    handle_mouse!(handle, MOUSE_BUTTON_EXTRA, out2);
    handle_mouse!(handle, MOUSE_BUTTON_FORWARD, out2);
    handle_mouse!(handle, MOUSE_BUTTON_BACK, out2);
    Input {
        codes: out,
        mouse: out2,
        mouse_x: handle.get_mouse_x(),
        mouse_y: handle.get_mouse_y(),
        mouse_dx: handle.get_mouse_delta().x,
        mouse_dy: handle.get_mouse_delta().y,
        scroll_amount: handle.get_mouse_wheel_move(),
    }
}
