use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tauri::{AppHandle, Runtime};

use crate::{start_recording, stop_recording};

trait HotkeyRegistry {
    fn register(&self, hotkey: HotKey) -> Result<(), String>;
    fn unregister(&self, hotkey: HotKey) -> Result<(), String>;
}

impl HotkeyRegistry for GlobalHotKeyManager {
    fn register(&self, hotkey: HotKey) -> Result<(), String> {
        GlobalHotKeyManager::register(self, hotkey).map_err(|_| {
            "That shortcut is unavailable. Choose another shortcut or close the app using it."
                .to_string()
        })
    }

    fn unregister(&self, hotkey: HotKey) -> Result<(), String> {
        GlobalHotKeyManager::unregister(self, hotkey).map_err(|error| error.to_string())
    }
}

struct HotkeyController<R> {
    registry: R,
    current: HotKey,
    active_id: Arc<AtomicU32>,
}

impl<R: HotkeyRegistry> HotkeyController<R> {
    fn replace_with<F>(&mut self, candidate: HotKey, persist: F) -> Result<Option<String>, String>
    where
        F: FnOnce(&str) -> Result<(), String>,
    {
        let normalized = candidate.to_string();
        if candidate == self.current {
            persist(&normalized)?;
            return Ok(None);
        }

        self.registry.register(candidate)?;
        if let Err(error) = persist(&normalized) {
            let _ = self.registry.unregister(candidate);
            return Err(error);
        }

        let previous = self.current;
        self.current = candidate;
        self.active_id.store(candidate.id(), Ordering::SeqCst);

        Ok(self.registry.unregister(previous).err())
    }
}

pub struct HotkeyHandle {
    controller: HotkeyController<GlobalHotKeyManager>,
}

unsafe impl Send for HotkeyHandle {}
unsafe impl Sync for HotkeyHandle {}

pub fn parse_shortcut(value: &str) -> Result<HotKey, String> {
    let hotkey = value
        .parse::<HotKey>()
        .map_err(|_| "Choose a valid keyboard shortcut".to_string())?;
    if hotkey.mods.is_empty() {
        return Err(
            "Shortcut must include a modifier: Ctrl, Alt, Shift, or the Windows key".to_string(),
        );
    }
    Ok(hotkey)
}

pub fn register<R: Runtime>(app: AppHandle<R>, shortcut: &str) -> Result<HotkeyHandle, String> {
    let hotkey = parse_shortcut(shortcut)?;
    let manager = GlobalHotKeyManager::new().map_err(|error| error.to_string())?;
    HotkeyRegistry::register(&manager, hotkey)?;
    let active_id = Arc::new(AtomicU32::new(hotkey.id()));
    let active_id_for_events = active_id.clone();
    let app_for_events = app.clone();

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id != active_id_for_events.load(Ordering::SeqCst) {
            return;
        }

        if event.state == HotKeyState::Pressed {
            let _ = start_recording(&app_for_events);
        } else if event.state == HotKeyState::Released {
            let _ = stop_recording(&app_for_events);
        }
    }));

    Ok(HotkeyHandle {
        controller: HotkeyController {
            registry: manager,
            current: hotkey,
            active_id,
        },
    })
}

impl HotkeyHandle {
    pub(crate) fn replace_with<F>(
        &mut self,
        shortcut: &str,
        persist: F,
    ) -> Result<Option<String>, String>
    where
        F: FnOnce(&str) -> Result<(), String>,
    {
        self.controller
            .replace_with(parse_shortcut(shortcut)?, persist)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    };

    use global_hotkey::hotkey::HotKey;

    use super::{parse_shortcut, HotkeyController, HotkeyRegistry};
    use crate::settings::DEFAULT_HOTKEY;

    #[derive(Default)]
    struct FakeState {
        registered: Vec<HotKey>,
        unregistered: Vec<HotKey>,
        fail_next_register: bool,
    }

    #[derive(Clone, Default)]
    struct FakeRegistry {
        state: Arc<Mutex<FakeState>>,
    }

    impl FakeRegistry {
        fn fail_next_register(&self) {
            self.state.lock().unwrap().fail_next_register = true;
        }

        fn was_unregistered(&self, hotkey: HotKey) -> bool {
            self.state.lock().unwrap().unregistered.contains(&hotkey)
        }
    }

    impl HotkeyRegistry for FakeRegistry {
        fn register(&self, hotkey: HotKey) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            if state.fail_next_register {
                state.fail_next_register = false;
                return Err("shortcut conflict".to_string());
            }
            state.registered.push(hotkey);
            Ok(())
        }

        fn unregister(&self, hotkey: HotKey) -> Result<(), String> {
            self.state.lock().unwrap().unregistered.push(hotkey);
            Ok(())
        }
    }

    fn test_controller() -> (HotkeyController<FakeRegistry>, FakeRegistry) {
        let registry = FakeRegistry::default();
        let current = parse_shortcut(DEFAULT_HOTKEY).unwrap();
        let active_id = Arc::new(AtomicU32::new(current.id()));
        (
            HotkeyController {
                registry: registry.clone(),
                current,
                active_id,
            },
            registry,
        )
    }

    #[test]
    fn parser_requires_a_modifier_and_normalizes_the_chord() {
        assert_eq!(
            parse_shortcut("Ctrl + Shift + D").unwrap().to_string(),
            "shift+control+KeyD"
        );
        assert!(parse_shortcut("D").unwrap_err().contains("modifier"));
    }

    #[test]
    fn rejected_candidate_keeps_the_old_active_shortcut() {
        let (mut controller, registry) = test_controller();
        registry.fail_next_register();
        let mut persisted = false;

        let result = controller.replace_with(parse_shortcut("Ctrl+Shift+D").unwrap(), |_| {
            persisted = true;
            Ok(())
        });

        assert!(result.is_err());
        assert!(!persisted);
        assert_eq!(controller.current.to_string(), DEFAULT_HOTKEY);
        assert_eq!(
            controller.active_id.load(Ordering::SeqCst),
            parse_shortcut(DEFAULT_HOTKEY).unwrap().id()
        );
    }

    #[test]
    fn failed_persistence_discards_candidate_and_keeps_old_active() {
        let (mut controller, registry) = test_controller();
        let candidate = parse_shortcut("Ctrl+Shift+D").unwrap();

        let result = controller.replace_with(candidate, |_| Err("disk full".to_string()));

        assert_eq!(result.unwrap_err(), "disk full");
        assert_eq!(controller.current.to_string(), DEFAULT_HOTKEY);
        assert_eq!(
            controller.active_id.load(Ordering::SeqCst),
            parse_shortcut(DEFAULT_HOTKEY).unwrap().id()
        );
        assert!(registry.was_unregistered(candidate));
    }

    #[test]
    fn successful_persistence_switches_active_id_and_cleans_old_registration() {
        let (mut controller, registry) = test_controller();
        let previous = controller.current;
        let candidate = parse_shortcut("Alt+KeyR").unwrap();
        let active_id = controller.active_id.clone();

        controller
            .replace_with(candidate, |_| {
                assert_eq!(active_id.load(Ordering::SeqCst), previous.id());
                Ok(())
            })
            .unwrap();

        assert_eq!(controller.current, candidate);
        assert_eq!(controller.active_id.load(Ordering::SeqCst), candidate.id());
        assert!(registry.was_unregistered(previous));
    }

    #[test]
    fn unchanged_shortcut_persists_without_reregistering() {
        let (mut controller, registry) = test_controller();
        let mut persisted = None;

        controller
            .replace_with(controller.current, |value| {
                persisted = Some(value.to_string());
                Ok(())
            })
            .unwrap();

        assert_eq!(persisted.as_deref(), Some(DEFAULT_HOTKEY));
        assert!(registry.state.lock().unwrap().registered.is_empty());
    }
}
