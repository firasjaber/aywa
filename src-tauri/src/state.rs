use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::APP_STATE_EVENT;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Saving,
    Error,
}

impl AppState {
    pub fn can_transition_to(self, next: AppState) -> bool {
        matches!(
            (self, next),
            (Self::Ready, Self::Listening)
                | (Self::Listening, Self::Saving)
                | (Self::Saving, Self::Ready)
                | (Self::Listening, Self::Error)
                | (Self::Saving, Self::Error)
                | (Self::Error, Self::Ready)
        )
    }

    pub fn default_message(self, error_message: Option<&str>) -> Option<String> {
        match self {
            Self::Ready => None,
            Self::Listening => Some("Listening...".to_string()),
            Self::Saving => Some("Saving recording...".to_string()),
            Self::Error => error_message.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub state: AppState,
    pub message: Option<String>,
}

pub struct AppStateManager {
    current: Mutex<AppState>,
}

impl Default for AppStateManager {
    fn default() -> Self {
        Self {
            current: Mutex::new(AppState::Ready),
        }
    }
}

impl AppStateManager {
    pub fn current(&self) -> AppState {
        *self.current.lock().expect("state mutex poisoned")
    }

    pub fn transition<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        next: AppState,
        message: Option<String>,
    ) -> Result<bool, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "state mutex poisoned".to_string())?;

        if !current.can_transition_to(next) {
            return Ok(false);
        }

        *current = next;
        app.emit(
            APP_STATE_EVENT,
            AppStatePayload {
                state: next,
                message: message.or_else(|| next.default_message(None)),
            },
        )
            .map_err(|error| error.to_string())?;

        Ok(true)
    }

    pub fn emit_current<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        let state = self.current();

        app.emit(
            APP_STATE_EVENT,
            AppStatePayload {
                state,
                message: state.default_message(None),
            },
        )
        .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub fn get_current_state(state: State<'_, AppStateManager>) -> AppStatePayload {
    let current = state.current();

    AppStatePayload {
        state: current,
        message: current.default_message(None),
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn ready_can_start_listening() {
        assert!(AppState::Ready.can_transition_to(AppState::Listening));
    }

    #[test]
    fn listening_can_start_saving() {
        assert!(AppState::Listening.can_transition_to(AppState::Saving));
    }

    #[test]
    fn saving_can_return_to_ready() {
        assert!(AppState::Saving.can_transition_to(AppState::Ready));
    }

    #[test]
    fn listening_can_fail() {
        assert!(AppState::Listening.can_transition_to(AppState::Error));
    }

    #[test]
    fn saving_can_fail() {
        assert!(AppState::Saving.can_transition_to(AppState::Error));
    }

    #[test]
    fn error_can_return_to_ready() {
        assert!(AppState::Error.can_transition_to(AppState::Ready));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!AppState::Ready.can_transition_to(AppState::Saving));
        assert!(!AppState::Ready.can_transition_to(AppState::Error));
        assert!(!AppState::Listening.can_transition_to(AppState::Ready));
        assert!(!AppState::Saving.can_transition_to(AppState::Listening));
        assert!(!AppState::Error.can_transition_to(AppState::Listening));
    }

    #[test]
    fn overlay_messages_match_phase_two_states() {
        assert_eq!(AppState::Ready.default_message(None), None);
        assert_eq!(
            AppState::Listening.default_message(None),
            Some("Listening...".to_string())
        );
        assert_eq!(
            AppState::Saving.default_message(None),
            Some("Saving recording...".to_string())
        );
        assert_eq!(
            AppState::Error.default_message(Some("No microphone")),
            Some("No microphone".to_string())
        );
    }
}
