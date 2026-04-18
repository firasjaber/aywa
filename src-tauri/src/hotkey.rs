use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tauri::{AppHandle, Runtime};

use crate::{start_recording, stop_recording};

pub struct HotkeyHandle {
    _manager: GlobalHotKeyManager,
    _hotkey: HotKey,
}

unsafe impl Send for HotkeyHandle {}
unsafe impl Sync for HotkeyHandle {}

pub fn register<R: Runtime>(app: AppHandle<R>) -> Result<HotkeyHandle, String> {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let manager = GlobalHotKeyManager::new().map_err(|error| error.to_string())?;

    manager.register(hotkey).map_err(|error| error.to_string())?;

    let app_for_events = app.clone();

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id != hotkey.id() {
            return;
        }

        if event.state == HotKeyState::Pressed {
            let _ = start_recording(&app_for_events);
        }

        if event.state == HotKeyState::Released {
            let _ = stop_recording(&app_for_events);
        }
    }));

    Ok(HotkeyHandle {
        _manager: manager,
        _hotkey: hotkey,
    })
}
