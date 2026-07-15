use tauri::{AppHandle, Manager, PhysicalPosition, Runtime};

use crate::{state::AppState, OVERLAY_WINDOW_LABEL};

const OVERLAY_MARGIN: i32 = 16;

pub fn configure<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) else {
        return Ok(());
    };

    window.set_ignore_cursor_events(true)?;
    position_overlay(&window)?;
    window.hide()?;

    Ok(())
}

pub fn show_overlay<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) else {
        return Ok(());
    };

    position_overlay(&window)?;
    window.show()?;

    Ok(())
}

pub fn hide_overlay<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) {
        window.hide()?;
    }

    Ok(())
}

pub fn sync_visibility_for_state<R: Runtime>(
    app: &AppHandle<R>,
    state: AppState,
) -> tauri::Result<()> {
    match state {
        AppState::Ready => hide_overlay(app),
        AppState::Listening | AppState::Processing | AppState::Completed | AppState::Error => {
            show_overlay(app)
        }
    }
}

pub fn toggle_overlay<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<bool> {
    let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) else {
        return Ok(false);
    };

    if window.is_visible()? {
        window.hide()?;
        return Ok(false);
    }

    position_overlay(&window)?;
    window.show()?;

    Ok(true)
}

fn position_overlay<R: Runtime>(window: &tauri::WebviewWindow<R>) -> tauri::Result<()> {
    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .or_else(|| window.available_monitors().ok()?.into_iter().next());

    let Some(monitor) = monitor else {
        return Ok(());
    };

    let work_area = monitor.work_area();
    let size = window.outer_size()?;
    let x = work_area.position.x + work_area.size.width as i32 - size.width as i32 - OVERLAY_MARGIN;
    let y =
        work_area.position.y + work_area.size.height as i32 - size.height as i32 - OVERLAY_MARGIN;

    window.set_position(PhysicalPosition::new(x, y))?;

    Ok(())
}
