// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};

use ok_lib::{
    model_manager::{
        service::{ModelManagerSnapshot, ModelManagerState},
        types::ModelId,
    },
    state::AppStateManager,
    RuntimeHandles,
};

const SETTINGS_MENU_ID: &str = "settings";
const QUIT_MENU_ID: &str = "quit";

fn build_tray<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, SETTINGS_MENU_ID, "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &separator, &quit])?;
    let icon = app
        .default_window_icon()
        .expect("default window icon should be configured")
        .clone();

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            SETTINGS_MENU_ID => {
                let _ = ok_lib::reveal_settings(app);
            }
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = ok_lib::reveal_settings(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
fn show_settings(app: AppHandle) -> Result<(), String> {
    ok_lib::show_settings(app)
}

#[tauri::command]
fn show_overlay(app: AppHandle) -> Result<(), String> {
    ok_lib::show_overlay(app)
}

#[tauri::command]
fn hide_overlay(app: AppHandle) -> Result<(), String> {
    ok_lib::hide_overlay(app)
}

#[tauri::command]
fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    ok_lib::toggle_overlay(app)
}

#[tauri::command]
fn get_current_state(state: State<'_, AppStateManager>) -> ok_lib::state::AppStatePayload {
    ok_lib::state::get_current_state(state)
}

#[tauri::command]
fn list_models(state: State<'_, ModelManagerState>) -> ModelManagerSnapshot {
    ok_lib::list_models(state.inner())
}

#[tauri::command]
fn select_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    ok_lib::select_model(&app, state.inner(), model_id)
}

#[tauri::command]
fn download_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    ok_lib::download_model(&app, state.inner(), model_id)
}

#[tauri::command]
fn load_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    ok_lib::load_model(&app, state.inner(), model_id)
}

#[tauri::command]
fn unload_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    ok_lib::unload_model(&app, state.inner())
}

#[tauri::command]
fn get_model_status(state: State<'_, ModelManagerState>) -> ModelManagerSnapshot {
    ok_lib::get_model_status(state.inner())
}

#[tauri::command]
fn clear_model_cache(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    ok_lib::clear_model_cache(&app, state.inner())
}

fn main() {
    tauri::Builder::default()
        .manage(AppStateManager::default())
        .manage(RuntimeHandles::default())
        .manage(ModelManagerState::default())
        .setup(|app| {
            ok_lib::configure_windows(&app.handle())?;
            build_tray(&app.handle())?;

            match ok_lib::hotkey::register(app.handle().clone()) {
                Ok(handle) => {
                    let runtime_handles: State<'_, RuntimeHandles> = app.state();
                    *runtime_handles.hotkey.lock().expect("hotkey mutex poisoned") = Some(handle);
                }
                Err(error) => {
                    eprintln!("failed to register global hotkey: {error}");
                }
            }

            let _ = app.state::<AppStateManager>().emit_current(&app.handle());
            if let Err(error) = ok_lib::startup_model_manager(&app.handle()) {
                eprintln!("failed to reconcile model manager state: {error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_settings,
            get_current_state,
            show_overlay,
            hide_overlay,
            toggle_overlay,
            list_models,
            select_model,
            download_model,
            load_model,
            unload_model,
            get_model_status,
            clear_model_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
