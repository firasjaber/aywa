use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tauri::{AppHandle, Runtime};

use crate::{run_fake_transcription_flow, state::AppState, transition_to};

pub struct HotkeyHandle {
    _manager: GlobalHotKeyManager,
    _hotkey: HotKey,
}

// This wrapper is only stored to keep the manager alive for the app lifetime.
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
            let _ = transition_to(&app_for_events, AppState::Listening);
        }

        if event.state == HotKeyState::Released
            && transition_to(&app_for_events, AppState::Processing).unwrap_or(false)
        {
            run_fake_transcription_flow(app_for_events.clone());
        }
    }));

    Ok(HotkeyHandle {
        _manager: manager,
        _hotkey: hotkey,
    })
}
