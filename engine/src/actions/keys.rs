use crate::config::MediaKey;
use std::mem::size_of;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

fn name_to_vk(name: &str) -> Option<VIRTUAL_KEY> {
    let n = name.to_ascii_lowercase();
    let vk = match n.as_str() {
        "ctrl" => VK_CONTROL,
        "shift" => VK_SHIFT,
        "alt" => VK_MENU,
        "win" => VK_LWIN,
        "enter" => VK_RETURN,
        "esc" => VK_ESCAPE,
        "tab" => VK_TAB,
        "space" => VK_SPACE,
        "backspace" => VK_BACK,
        "delete" => VK_DELETE,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        _ => {
            let b = n.as_bytes();
            if b.len() == 1 && b[0].is_ascii_alphanumeric() {
                // VK codes for 0-9 and A-Z equal their ASCII uppercase.
                return Some(VIRTUAL_KEY(b[0].to_ascii_uppercase() as u16));
            }
            let f: u16 = n.strip_prefix('f')?.parse().ok()?;
            if !(1..=24).contains(&f) {
                return None;
            }
            return Some(VIRTUAL_KEY(VK_F1.0 + f - 1));
        }
    };
    Some(vk)
}

fn is_modifier(vk: VIRTUAL_KEY) -> bool {
    [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN].contains(&vk)
}

/// Modifiers first (Ctrl, Shift, Alt, Win), then the other keys in written order.
pub fn parse_shortcut(s: &str) -> Result<Vec<VIRTUAL_KEY>, String> {
    let mut keys = Vec::new();
    for part in s.split('+') {
        let part = part.trim();
        if part.is_empty() {
            return Err(format!("raccourci invalide : {s:?}"));
        }
        keys.push(name_to_vk(part).ok_or_else(|| format!("touche inconnue : {part}"))?);
    }
    let mut out: Vec<_> = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN]
        .into_iter()
        .filter(|m| keys.contains(m))
        .collect();
    out.extend(keys.into_iter().filter(|k| !is_modifier(*k)));
    Ok(out)
}

fn key_input(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    if inputs.is_empty() {
        return;
    }
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) } as usize;
    if sent < inputs.len() {
        eprintln!("SendInput: {sent}/{} evenements injectes", inputs.len());
    }
}

fn is_extended(vk: VIRTUAL_KEY) -> bool {
    [
        VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_DELETE, VK_LWIN,
    ]
    .contains(&vk)
}

/// All key-downs, then key-ups in reverse order, in a single SendInput call.
pub fn send_shortcut(keys: &[VIRTUAL_KEY]) {
    let flags = |vk, up: bool| {
        let mut f = if up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        };
        if is_extended(vk) {
            f |= KEYEVENTF_EXTENDEDKEY;
        }
        f
    };
    let inputs: Vec<INPUT> = keys
        .iter()
        .map(|&k| key_input(k, 0, flags(k, false)))
        .chain(keys.iter().rev().map(|&k| key_input(k, 0, flags(k, true))))
        .collect();
    send(&inputs);
}

/// Types text as Unicode (UTF-16 units, so surrogate pairs work) in a single SendInput call.
pub fn send_text(text: &str) {
    let inputs: Vec<INPUT> = text
        .encode_utf16()
        .flat_map(|u| {
            [
                key_input(VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE),
                key_input(VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP),
            ]
        })
        .collect();
    send(&inputs);
}

pub fn send_media(key: MediaKey) {
    send_shortcut(&[match key {
        MediaKey::PlayPause => VK_MEDIA_PLAY_PAUSE,
        MediaKey::Next => VK_MEDIA_NEXT_TRACK,
        MediaKey::Prev => VK_MEDIA_PREV_TRACK,
        MediaKey::Stop => VK_MEDIA_STOP,
    }]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ctrl_shift_s() {
        assert_eq!(
            parse_shortcut("ctrl+Shift+S").unwrap(),
            [VK_CONTROL, VK_SHIFT, VK_S]
        );
    }

    #[test]
    fn modifiers_first() {
        assert_eq!(parse_shortcut("S+Ctrl").unwrap(), [VK_CONTROL, VK_S]);
    }

    #[test]
    fn rejects_unknown() {
        assert!(parse_shortcut("Ctrl+Banane")
            .unwrap_err()
            .contains("Banane"));
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_shortcut("").is_err());
        assert!(parse_shortcut("Ctrl+").is_err());
    }

    #[test]
    #[ignore = "injects real keystrokes"]
    fn manual_types_text() {
        std::thread::sleep(std::time::Duration::from_secs(3));
        send_text("é✓ ok");
    }
}
