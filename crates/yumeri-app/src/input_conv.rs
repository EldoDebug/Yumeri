use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::keyboard::{Key as WinitKey, KeyCode as WinitKeyCode, NamedKey as WinitNamedKey, PhysicalKey};
use yumeri_input::{
    ButtonState, InputEvent, Key, KeyCode, KeyboardEvent, Modifiers, MouseButton, NamedKey,
    PointerEvent, PointerEventKind,
};

pub fn convert_window_event(
    event: &WindowEvent,
    modifiers: &winit::event::Modifiers,
    cursor_position: (f64, f64),
) -> Option<InputEvent> {
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            let key = convert_key(&event.logical_key);
            let code = convert_key_code(&event.physical_key);
            let state = convert_element_state(event.state);
            let mods = convert_modifiers(modifiers);
            let text = if event.state.is_pressed() {
                event.text.as_ref().map(|s| s.to_string())
            } else {
                None
            };
            Some(InputEvent::Keyboard(KeyboardEvent {
                key,
                code,
                state,
                modifiers: mods,
                text,
                repeat: event.repeat,
            }))
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let btn = convert_mouse_button(*button);
            let kind = match state {
                ElementState::Pressed => PointerEventKind::ButtonPressed(btn),
                ElementState::Released => PointerEventKind::ButtonReleased(btn),
            };
            let mods = convert_modifiers(modifiers);
            Some(InputEvent::Pointer(PointerEvent {
                kind,
                position: cursor_position,
                modifiers: mods,
            }))
        }
        WindowEvent::CursorMoved { position, .. } => {
            let mods = convert_modifiers(modifiers);
            Some(InputEvent::Pointer(PointerEvent {
                kind: PointerEventKind::Moved,
                position: (position.x, position.y),
                modifiers: mods,
            }))
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let (dx, dy) = match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => (*x as f64, *y as f64),
                winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
            };
            let mods = convert_modifiers(modifiers);
            Some(InputEvent::Pointer(PointerEvent {
                kind: PointerEventKind::Scroll {
                    delta_x: dx,
                    delta_y: dy,
                },
                position: cursor_position,
                modifiers: mods,
            }))
        }
        WindowEvent::CursorEntered { .. } => {
            let mods = convert_modifiers(modifiers);
            Some(InputEvent::Pointer(PointerEvent {
                kind: PointerEventKind::Entered,
                position: cursor_position,
                modifiers: mods,
            }))
        }
        WindowEvent::CursorLeft { .. } => {
            let mods = convert_modifiers(modifiers);
            Some(InputEvent::Pointer(PointerEvent {
                kind: PointerEventKind::Left,
                position: cursor_position,
                modifiers: mods,
            }))
        }
        WindowEvent::Focused(focused) => Some(InputEvent::FocusChanged(*focused)),
        _ => None,
    }
}

fn convert_key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Named(named) => match convert_named_key(*named) {
            Some(n) => Key::Named(n),
            None => Key::Unidentified,
        },
        WinitKey::Character(c) => Key::Character(c.to_string()),
        _ => Key::Unidentified,
    }
}

fn convert_named_key(key: WinitNamedKey) -> Option<NamedKey> {
    Some(match key {
        WinitNamedKey::Enter => NamedKey::Enter,
        WinitNamedKey::Tab => NamedKey::Tab,
        WinitNamedKey::Space => NamedKey::Space,
        WinitNamedKey::Backspace => NamedKey::Backspace,
        WinitNamedKey::Delete => NamedKey::Delete,
        WinitNamedKey::Escape => NamedKey::Escape,
        WinitNamedKey::ArrowUp => NamedKey::ArrowUp,
        WinitNamedKey::ArrowDown => NamedKey::ArrowDown,
        WinitNamedKey::ArrowLeft => NamedKey::ArrowLeft,
        WinitNamedKey::ArrowRight => NamedKey::ArrowRight,
        WinitNamedKey::Home => NamedKey::Home,
        WinitNamedKey::End => NamedKey::End,
        WinitNamedKey::PageUp => NamedKey::PageUp,
        WinitNamedKey::PageDown => NamedKey::PageDown,
        WinitNamedKey::Insert => NamedKey::Insert,
        WinitNamedKey::CapsLock => NamedKey::CapsLock,
        WinitNamedKey::NumLock => NamedKey::NumLock,
        WinitNamedKey::ScrollLock => NamedKey::ScrollLock,
        WinitNamedKey::PrintScreen => NamedKey::PrintScreen,
        WinitNamedKey::Pause => NamedKey::Pause,
        WinitNamedKey::F1 => NamedKey::F1,
        WinitNamedKey::F2 => NamedKey::F2,
        WinitNamedKey::F3 => NamedKey::F3,
        WinitNamedKey::F4 => NamedKey::F4,
        WinitNamedKey::F5 => NamedKey::F5,
        WinitNamedKey::F6 => NamedKey::F6,
        WinitNamedKey::F7 => NamedKey::F7,
        WinitNamedKey::F8 => NamedKey::F8,
        WinitNamedKey::F9 => NamedKey::F9,
        WinitNamedKey::F10 => NamedKey::F10,
        WinitNamedKey::F11 => NamedKey::F11,
        WinitNamedKey::F12 => NamedKey::F12,
        WinitNamedKey::Shift => NamedKey::Shift,
        WinitNamedKey::Control => NamedKey::Control,
        WinitNamedKey::Alt => NamedKey::Alt,
        WinitNamedKey::Meta => NamedKey::Meta,
        _ => return None,
    })
}

fn convert_key_code(physical_key: &PhysicalKey) -> KeyCode {
    match physical_key {
        PhysicalKey::Code(code) => match code {
            WinitKeyCode::KeyA => KeyCode::KeyA,
            WinitKeyCode::KeyB => KeyCode::KeyB,
            WinitKeyCode::KeyC => KeyCode::KeyC,
            WinitKeyCode::KeyD => KeyCode::KeyD,
            WinitKeyCode::KeyE => KeyCode::KeyE,
            WinitKeyCode::KeyF => KeyCode::KeyF,
            WinitKeyCode::KeyG => KeyCode::KeyG,
            WinitKeyCode::KeyH => KeyCode::KeyH,
            WinitKeyCode::KeyI => KeyCode::KeyI,
            WinitKeyCode::KeyJ => KeyCode::KeyJ,
            WinitKeyCode::KeyK => KeyCode::KeyK,
            WinitKeyCode::KeyL => KeyCode::KeyL,
            WinitKeyCode::KeyM => KeyCode::KeyM,
            WinitKeyCode::KeyN => KeyCode::KeyN,
            WinitKeyCode::KeyO => KeyCode::KeyO,
            WinitKeyCode::KeyP => KeyCode::KeyP,
            WinitKeyCode::KeyQ => KeyCode::KeyQ,
            WinitKeyCode::KeyR => KeyCode::KeyR,
            WinitKeyCode::KeyS => KeyCode::KeyS,
            WinitKeyCode::KeyT => KeyCode::KeyT,
            WinitKeyCode::KeyU => KeyCode::KeyU,
            WinitKeyCode::KeyV => KeyCode::KeyV,
            WinitKeyCode::KeyW => KeyCode::KeyW,
            WinitKeyCode::KeyX => KeyCode::KeyX,
            WinitKeyCode::KeyY => KeyCode::KeyY,
            WinitKeyCode::KeyZ => KeyCode::KeyZ,
            WinitKeyCode::Digit0 => KeyCode::Digit0,
            WinitKeyCode::Digit1 => KeyCode::Digit1,
            WinitKeyCode::Digit2 => KeyCode::Digit2,
            WinitKeyCode::Digit3 => KeyCode::Digit3,
            WinitKeyCode::Digit4 => KeyCode::Digit4,
            WinitKeyCode::Digit5 => KeyCode::Digit5,
            WinitKeyCode::Digit6 => KeyCode::Digit6,
            WinitKeyCode::Digit7 => KeyCode::Digit7,
            WinitKeyCode::Digit8 => KeyCode::Digit8,
            WinitKeyCode::Digit9 => KeyCode::Digit9,
            WinitKeyCode::F1 => KeyCode::F1,
            WinitKeyCode::F2 => KeyCode::F2,
            WinitKeyCode::F3 => KeyCode::F3,
            WinitKeyCode::F4 => KeyCode::F4,
            WinitKeyCode::F5 => KeyCode::F5,
            WinitKeyCode::F6 => KeyCode::F6,
            WinitKeyCode::F7 => KeyCode::F7,
            WinitKeyCode::F8 => KeyCode::F8,
            WinitKeyCode::F9 => KeyCode::F9,
            WinitKeyCode::F10 => KeyCode::F10,
            WinitKeyCode::F11 => KeyCode::F11,
            WinitKeyCode::F12 => KeyCode::F12,
            WinitKeyCode::Space => KeyCode::Space,
            WinitKeyCode::Enter => KeyCode::Enter,
            WinitKeyCode::Tab => KeyCode::Tab,
            WinitKeyCode::Backspace => KeyCode::Backspace,
            WinitKeyCode::Delete => KeyCode::Delete,
            WinitKeyCode::Escape => KeyCode::Escape,
            WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
            WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
            WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
            WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
            WinitKeyCode::Home => KeyCode::Home,
            WinitKeyCode::End => KeyCode::End,
            WinitKeyCode::PageUp => KeyCode::PageUp,
            WinitKeyCode::PageDown => KeyCode::PageDown,
            WinitKeyCode::Insert => KeyCode::Insert,
            WinitKeyCode::CapsLock => KeyCode::CapsLock,
            WinitKeyCode::NumLock => KeyCode::NumLock,
            WinitKeyCode::ScrollLock => KeyCode::ScrollLock,
            WinitKeyCode::PrintScreen => KeyCode::PrintScreen,
            WinitKeyCode::Pause => KeyCode::Pause,
            WinitKeyCode::ShiftLeft => KeyCode::ShiftLeft,
            WinitKeyCode::ShiftRight => KeyCode::ShiftRight,
            WinitKeyCode::ControlLeft => KeyCode::ControlLeft,
            WinitKeyCode::ControlRight => KeyCode::ControlRight,
            WinitKeyCode::AltLeft => KeyCode::AltLeft,
            WinitKeyCode::AltRight => KeyCode::AltRight,
            WinitKeyCode::SuperLeft => KeyCode::MetaLeft,
            WinitKeyCode::SuperRight => KeyCode::MetaRight,
            WinitKeyCode::Semicolon => KeyCode::Semicolon,
            WinitKeyCode::Equal => KeyCode::Equal,
            WinitKeyCode::Comma => KeyCode::Comma,
            WinitKeyCode::Minus => KeyCode::Minus,
            WinitKeyCode::Period => KeyCode::Period,
            WinitKeyCode::Slash => KeyCode::Slash,
            WinitKeyCode::Backquote => KeyCode::Backquote,
            WinitKeyCode::BracketLeft => KeyCode::BracketLeft,
            WinitKeyCode::Backslash => KeyCode::Backslash,
            WinitKeyCode::BracketRight => KeyCode::BracketRight,
            WinitKeyCode::Quote => KeyCode::Quote,
            _ => KeyCode::Other(0),
        },
        PhysicalKey::Unidentified(_) => KeyCode::Other(0),
    }
}

fn convert_element_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

fn convert_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(id) => MouseButton::Other(id),
    }
}

fn convert_modifiers(mods: &winit::event::Modifiers) -> Modifiers {
    let state = mods.state();
    Modifiers {
        shift: state.shift_key(),
        ctrl: state.control_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}
