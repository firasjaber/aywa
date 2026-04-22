use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn copy_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text.to_string())
        .map_err(|error| error.to_string())
}

fn paste_shortcut_sequence() -> Vec<(&'static str, &'static str)> {
    vec![("down", "ctrl"), ("press", "v"), ("up", "ctrl")]
}

pub fn paste_clipboard() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;
    for (action, key) in paste_shortcut_sequence() {
        match (action, key) {
            ("down", "ctrl") => enigo
                .key(Key::Control, Direction::Press)
                .map_err(|error| error.to_string())?,
            ("press", "v") => enigo
                .key(Key::Unicode('v'), Direction::Click)
                .map_err(|error| error.to_string())?,
            ("up", "ctrl") => enigo
                .key(Key::Control, Direction::Release)
                .map_err(|error| error.to_string())?,
            _ => return Err(format!("unsupported paste action: {action}:{key}")),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::paste_shortcut_sequence;

    #[test]
    fn paste_shortcut_sequence_uses_ctrl_v() {
        assert_eq!(
            paste_shortcut_sequence(),
            vec![("down", "ctrl"), ("press", "v"), ("up", "ctrl")]
        );
    }
}
