use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::APP_STATE_EVENT;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Processing,
    Completed,
}

impl AppState {
    pub fn can_transition_to(self, next: AppState) -> bool {
        matches!(
            (self, next),
            (Self::Ready, Self::Listening)
                | (Self::Listening, Self::Processing)
                | (Self::Processing, Self::Completed)
                | (Self::Completed, Self::Ready)
        )
    }

    pub fn overlay_message(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Listening => Some("Listening..."),
            Self::Processing => Some("Transcribing..."),
            Self::Completed => Some("Copied to clipboard"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub state: AppState,
    pub message: Option<&'static str>,
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

    pub fn transition<R: Runtime>(&self, app: &AppHandle<R>, next: AppState) -> Result<bool, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "state mutex poisoned".to_string())?;

        if !current.can_transition_to(next) {
            return Ok(false);
        }

        *current = next;
        let payload = AppStatePayload {
            state: next,
            message: next.overlay_message(),
        };

        app.emit(APP_STATE_EVENT, payload)
            .map_err(|error| error.to_string())?;

        Ok(true)
    }

    pub fn emit_current<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        let state = self.current();

        app.emit(
            APP_STATE_EVENT,
            AppStatePayload {
                state,
                message: state.overlay_message(),
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
        message: current.overlay_message(),
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
    fn listening_can_start_processing() {
        assert!(AppState::Listening.can_transition_to(AppState::Processing));
    }

    #[test]
    fn processing_can_complete() {
        assert!(AppState::Processing.can_transition_to(AppState::Completed));
    }

    #[test]
    fn completed_can_return_to_ready() {
        assert!(AppState::Completed.can_transition_to(AppState::Ready));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!AppState::Ready.can_transition_to(AppState::Processing));
        assert!(!AppState::Listening.can_transition_to(AppState::Completed));
        assert!(!AppState::Processing.can_transition_to(AppState::Listening));
        assert!(!AppState::Completed.can_transition_to(AppState::Processing));
    }
}
