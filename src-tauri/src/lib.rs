use std::{fs, path::PathBuf, str::FromStr, sync::Mutex, thread, time::Duration};

#[cfg(target_os = "macos")]
use core_graphics::{
    event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, KeyCode},
    event_source::{CGEventSource, CGEventSourceStateID},
    geometry::CGPoint,
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;
use serde::{Deserialize, Serialize};
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuId, SubmenuBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, Window,
};
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, Shortcut, ShortcutState,
};

const OVERLAY_LABEL: &str = "overlay";
const SETTINGS_LABEL: &str = "settings";
const TRAY_ID: &str = "main";
const SHOW_MENU_ID: &str = "show_whip";
const SETTINGS_MENU_ID: &str = "text_settings";
const QUIT_MENU_ID: &str = "quit";
const DEFAULT_PROMPT_TEXT: &str = "快点，加油";
const SETTINGS_FILE_NAME: &str = "settings.json";
const SHOW_WHIP_SHORTCUT: &str = "Shift+1";
const SEND_PROMPT_SHORTCUT: &str = "Shift+2";
const HIDE_WHIP_SHORTCUT: &str = "Shift+3";
const CRACK_SEND_MODE_SELECTED_INPUT: &str = "selectedInput";
const CRACK_SEND_MODE_TARGET_ZONES: &str = "targetZones";
const LANGUAGE_ZH_CN: &str = "zh-CN";
const LANGUAGE_EN_US: &str = "en-US";

#[derive(Default)]
struct OverlayLifecycle {
    ready: bool,
    pending_spawn: bool,
}

#[derive(Default)]
struct PreviousAppState {
    bundle_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TargetBox {
    id: String,
    name: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    prompt_text: String,
}

impl Default for TargetBox {
    fn default() -> Self {
        Self {
            id: "target-1".to_string(),
            name: "Target 1".to_string(),
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 12.0,
            prompt_text: String::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PromptSettings {
    prompt_text: String,
    show_whip_shortcut: String,
    send_prompt_shortcut: String,
    hide_whip_shortcut: String,
    language: String,
    crack_send_mode: String,
    targets: Vec<TargetBox>,
}

impl Default for PromptSettings {
    fn default() -> Self {
        Self {
            prompt_text: DEFAULT_PROMPT_TEXT.to_string(),
            show_whip_shortcut: SHOW_WHIP_SHORTCUT.to_string(),
            send_prompt_shortcut: SEND_PROMPT_SHORTCUT.to_string(),
            hide_whip_shortcut: HIDE_WHIP_SHORTCUT.to_string(),
            language: LANGUAGE_ZH_CN.to_string(),
            crack_send_mode: CRACK_SEND_MODE_SELECTED_INPUT.to_string(),
            targets: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
struct SpawnWhipPayload {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Deserialize)]
struct CrackPayload {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]
struct GlobalPoint {
    x: f64,
    y: f64,
}

enum CrackAction {
    ClickAtPoint { point: GlobalPoint, text: String },
    SendToPreviousSelection { text: String },
}

#[tauri::command]
fn whip_crack(
    window: Window,
    payload: CrackPayload,
    settings: State<'_, Mutex<PromptSettings>>,
) -> Result<(), String> {
    let origin = window.inner_position().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let action = {
        let settings = settings
            .lock()
            .map_err(|_| "failed to lock prompt settings".to_string())?;
        resolve_whip_action(
            &settings,
            payload,
            f64::from(origin.x),
            f64::from(origin.y),
            f64::from(size.width),
            f64::from(size.height),
        )
    };
    let app = window.app_handle().clone();

    thread::spawn(move || {
        let result = match action {
            CrackAction::ClickAtPoint { point, text } => send_macro_to_point(&app, point, &text),
            CrackAction::SendToPreviousSelection { text } => {
                send_prompt_to_previous_selection(&app, &text)
            }
        };

        if let Err(error) = result {
            log::warn!("failed to send whip prompt: {error}");
        }
    });

    Ok(())
}

#[tauri::command]
fn get_prompt_settings(
    settings: State<'_, Mutex<PromptSettings>>,
) -> Result<PromptSettings, String> {
    settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "failed to lock prompt settings".to_string())
}

#[tauri::command]
fn set_prompt_text(
    app: AppHandle,
    settings: State<'_, Mutex<PromptSettings>>,
    text: String,
) -> Result<PromptSettings, String> {
    let current = settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "failed to lock prompt settings".to_string())?;
    let updated = normalize_prompt_settings(PromptSettings {
        prompt_text: text,
        ..current
    })?;

    {
        let mut settings = settings
            .lock()
            .map_err(|_| "failed to lock prompt settings".to_string())?;
        *settings = updated.clone();
    }

    save_prompt_settings(&app, &updated)?;
    let _ = app.emit("prompt-settings-updated", updated.clone());
    Ok(updated)
}

#[tauri::command]
fn set_prompt_settings(
    app: AppHandle,
    settings: State<'_, Mutex<PromptSettings>>,
    prompt_text: String,
    show_whip_shortcut: String,
    send_prompt_shortcut: String,
    hide_whip_shortcut: String,
    language: String,
    crack_send_mode: String,
    targets: Vec<TargetBox>,
) -> Result<PromptSettings, String> {
    let previous = settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "failed to lock prompt settings".to_string())?;
    let updated = normalize_prompt_settings(PromptSettings {
        prompt_text,
        show_whip_shortcut,
        send_prompt_shortcut,
        hide_whip_shortcut,
        language,
        crack_send_mode,
        targets,
    })?;

    sync_global_shortcuts(&app, Some(&previous), &updated)?;

    {
        let mut settings = settings
            .lock()
            .map_err(|_| "failed to lock prompt settings".to_string())?;
        *settings = updated.clone();
    }

    refresh_localized_shell(&app, &updated).map_err(|error| error.to_string())?;
    save_prompt_settings(&app, &updated)?;
    let _ = app.emit("prompt-settings-updated", updated.clone());
    Ok(updated)
}

#[tauri::command]
fn hide_overlay(window: Window) -> Result<(), String> {
    let _ = window.emit("overlay-hidden", ());
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.set_focusable(true);
    window.hide().map_err(|error| error.to_string())?;
    refocus_previous_app(&window.app_handle());
    Ok(())
}

fn hide_overlay_for_app(app: &AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };

    let _ = window.emit("overlay-hidden", ());
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.set_focusable(true);
    let _ = window.hide();
    refocus_previous_app(app);
}

#[tauri::command]
fn set_overlay_passthrough(window: Window, passthrough: bool) -> Result<(), String> {
    window
        .set_focusable(!passthrough)
        .map_err(|error| error.to_string())?;
    window
        .set_ignore_cursor_events(passthrough)
        .map_err(|error| error.to_string())?;

    if passthrough {
        refocus_previous_app(&window.app_handle());
    } else {
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
fn send_prompt_to_focused(
    app: AppHandle,
    settings: State<'_, Mutex<PromptSettings>>,
) -> Result<(), String> {
    let prompt_text = {
        let settings = settings
            .lock()
            .map_err(|_| "failed to lock prompt settings".to_string())?;
        normalize_prompt_text(&settings.prompt_text)
    };

    thread::spawn(move || {
        if let Err(error) = send_prompt_to_previous_selection(&app, &prompt_text) {
            log::warn!("failed to send prompt to previous selection: {error}");
        }
    });

    Ok(())
}

#[tauri::command]
fn overlay_ready(
    app: AppHandle,
    lifecycle: State<'_, Mutex<OverlayLifecycle>>,
) -> Result<(), String> {
    let mut lifecycle = lifecycle
        .lock()
        .map_err(|_| "failed to lock overlay lifecycle".to_string())?;

    lifecycle.ready = true;

    if lifecycle.pending_spawn {
        lifecycle.pending_spawn = false;
        if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
            emit_spawn_whip(&window);
        }
    }

    Ok(())
}

fn build_tray_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let texts = localized_texts(language);
    MenuBuilder::new(app)
        .text(SHOW_MENU_ID, texts.show_whip)
        .text(SETTINGS_MENU_ID, texts.settings)
        .separator()
        .text(QUIT_MENU_ID, texts.quit)
        .build()
}

fn build_tray(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let texts = localized_texts(language);
    let menu = build_tray_menu(app, language)?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(texts.tray_tooltip)
        .show_menu_on_left_click(false);

    #[cfg(target_os = "macos")]
    {
        let template_icon =
            Image::from_bytes(include_bytes!("../icons/trayTemplate.png"))?.to_owned();
        tray = tray.icon(template_icon).icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.on_menu_event(|app, event| {
        handle_menu_action(app, event.id());
    })
    .on_tray_icon_event(|tray, event| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            }
        ) {
            toggle_overlay(tray.app_handle());
        }
    })
    .build(app)?;

    Ok(())
}

fn build_app_menu(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let texts = localized_texts(language);
    let menu = Menu::new(app)?;
    let app_submenu = SubmenuBuilder::new(app, texts.app_menu)
        .text(SHOW_MENU_ID, texts.show_whip)
        .text(SETTINGS_MENU_ID, texts.settings)
        .separator()
        .quit()
        .build()?;
    let edit_submenu = SubmenuBuilder::new(app, texts.edit_menu)
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    menu.append(&app_submenu)?;
    menu.append(&edit_submenu)?;
    app.set_menu(menu)?;

    Ok(())
}

fn refresh_localized_shell(app: &AppHandle, settings: &PromptSettings) -> tauri::Result<()> {
    let texts = localized_texts(&settings.language);

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_tray_menu(app, &settings.language)?))?;
        tray.set_tooltip(Some(texts.tray_tooltip))?;
    }

    build_app_menu(app, &settings.language)?;

    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.set_title(texts.settings_title);
    }

    Ok(())
}

fn create_overlay_window(app: &AppHandle) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == OVERLAY_LABEL);

    if let Some(config) = config {
        let window = tauri::WebviewWindowBuilder::from_config(app, config)?.build()?;
        configure_overlay_window(app, &window)?;
        window.hide()?;
    }

    Ok(())
}

fn show_settings_window(app: &AppHandle) -> tauri::Result<()> {
    let language = app
        .state::<Mutex<PromptSettings>>()
        .lock()
        .map(|settings| settings.language.clone())
        .unwrap_or_else(|_| LANGUAGE_ZH_CN.to_string());
    let texts = localized_texts(&language);

    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.set_title(texts.settings_title);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let window =
        tauri::WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
            .title(texts.settings_title)
            .inner_size(1040.0, 820.0)
            .min_inner_size(860.0, 720.0)
            .resizable(true)
            .maximizable(false)
            .minimizable(false)
            .always_on_top(true)
            .build()?;

    let _ = window.center();
    let _ = window.set_focus();

    Ok(())
}

fn configure_overlay_window(app: &AppHandle, window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let monitor = overlay_target_monitor(window)?.or(app.primary_monitor()?);

    if let Some(monitor) = monitor {
        window.set_position(*monitor.position())?;
        window.set_size(*monitor.size())?;
    }

    let _ = window.set_always_on_top(true);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_shadow(false);

    #[cfg(not(target_os = "windows"))]
    let _ = window.set_visible_on_all_workspaces(true);

    Ok(())
}

fn overlay_cursor_position(window: &tauri::WebviewWindow) -> Option<PhysicalPosition<f64>> {
    window.cursor_position().ok()
}

fn overlay_target_monitor(window: &tauri::WebviewWindow) -> tauri::Result<Option<tauri::Monitor>> {
    if let Some(cursor) = overlay_cursor_position(window) {
        if let Some(monitor) = window.monitor_from_point(cursor.x, cursor.y)? {
            return Ok(Some(monitor));
        }
    }

    window.current_monitor()
}

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn overlay_cursor_payload(window: &tauri::WebviewWindow) -> SpawnWhipPayload {
    let default = window
        .inner_size()
        .map(|size| SpawnWhipPayload {
            x: f64::from(size.width) / 2.0,
            y: f64::from(size.height) / 2.0,
        })
        .unwrap_or(SpawnWhipPayload { x: 720.0, y: 450.0 });

    let Some(cursor) = overlay_cursor_position(window) else {
        return default;
    };
    let Ok(origin) = window.inner_position() else {
        return default;
    };
    let Ok(size) = window.inner_size() else {
        return default;
    };

    let max_x = f64::from(size.width.saturating_sub(1));
    let max_y = f64::from(size.height.saturating_sub(1));

    SpawnWhipPayload {
        x: clamp_f64(cursor.x - f64::from(origin.x), 0.0, max_x),
        y: clamp_f64(cursor.y - f64::from(origin.y), 0.0, max_y),
    }
}

fn emit_spawn_whip(window: &tauri::WebviewWindow) {
    let _ = window.emit("spawn-whip", overlay_cursor_payload(window));
}

fn normalize_prompt_text(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        DEFAULT_PROMPT_TEXT.to_string()
    } else {
        text.to_string()
    }
}

fn normalize_target_prompt_text(text: &str) -> String {
    text.trim().to_string()
}

fn normalize_language(value: &str) -> String {
    match value.trim() {
        LANGUAGE_EN_US | "en" => LANGUAGE_EN_US.to_string(),
        _ => LANGUAGE_ZH_CN.to_string(),
    }
}

fn is_english(language: &str) -> bool {
    language == LANGUAGE_EN_US
}

fn normalize_crack_send_mode(value: &str) -> String {
    match value.trim() {
        CRACK_SEND_MODE_TARGET_ZONES => CRACK_SEND_MODE_TARGET_ZONES.to_string(),
        _ => CRACK_SEND_MODE_SELECTED_INPUT.to_string(),
    }
}

fn normalize_shortcut(value: &str, fallback: &str, language: &str) -> Result<String, String> {
    let shortcut = if value.trim().is_empty() {
        fallback.trim()
    } else {
        value.trim()
    };

    Shortcut::from_str(shortcut)
        .map(|_| shortcut.to_string())
        .map_err(|error| {
            if is_english(language) {
                format!("Invalid shortcut format: {shortcut} ({error})")
            } else {
                format!("快捷键格式无效：{shortcut} ({error})")
            }
        })
}

fn normalize_target_box(target: TargetBox, index: usize, language: &str) -> Result<TargetBox, String> {
    let id = if target.id.trim().is_empty() {
        format!("target-{}", index + 1)
    } else {
        target.id.trim().to_string()
    };
    let name = if target.name.trim().is_empty() {
        if is_english(language) {
            format!("Target {}", index + 1)
        } else {
            format!("目标 {}", index + 1)
        }
    } else {
        target.name.trim().to_string()
    };

    let values = [
        ("x", target.x),
        ("y", target.y),
        ("width", target.width),
        ("height", target.height),
    ];

    for (field, value) in values {
        if !value.is_finite() {
            return Err(if is_english(language) {
                format!("{name} has an invalid {field} value")
            } else {
                format!("{name} 的 {field} 不是有效数字")
            });
        }
    }

    if target.x < 0.0 || target.x > 100.0 {
        return Err(if is_english(language) {
            format!("{name}: x must stay between 0 and 100")
        } else {
            format!("{name} 的 x 必须在 0 到 100 之间")
        });
    }

    if target.y < 0.0 || target.y > 100.0 {
        return Err(if is_english(language) {
            format!("{name}: y must stay between 0 and 100")
        } else {
            format!("{name} 的 y 必须在 0 到 100 之间")
        });
    }

    if target.width <= 0.0 || target.width > 100.0 {
        return Err(if is_english(language) {
            format!("{name}: width must be greater than 0 and no more than 100")
        } else {
            format!("{name} 的 width 必须大于 0 且不超过 100")
        });
    }

    if target.height <= 0.0 || target.height > 100.0 {
        return Err(if is_english(language) {
            format!("{name}: height must be greater than 0 and no more than 100")
        } else {
            format!("{name} 的 height 必须大于 0 且不超过 100")
        });
    }

    if target.x + target.width > 100.0 {
        return Err(if is_english(language) {
            format!("{name}: x + width cannot exceed 100")
        } else {
            format!("{name} 的 x + width 不能超过 100")
        });
    }

    if target.y + target.height > 100.0 {
        return Err(if is_english(language) {
            format!("{name}: y + height cannot exceed 100")
        } else {
            format!("{name} 的 y + height 不能超过 100")
        });
    }

    Ok(TargetBox {
        id,
        name,
        x: target.x,
        y: target.y,
        width: target.width,
        height: target.height,
        prompt_text: normalize_target_prompt_text(&target.prompt_text),
    })
}

fn normalize_prompt_settings(settings: PromptSettings) -> Result<PromptSettings, String> {
    let language = normalize_language(&settings.language);
    let targets = settings
        .targets
        .into_iter()
        .enumerate()
        .map(|(index, target)| normalize_target_box(target, index, &language))
        .collect::<Result<Vec<_>, _>>()?;
    let normalized = PromptSettings {
        prompt_text: normalize_prompt_text(&settings.prompt_text),
        show_whip_shortcut: normalize_shortcut(
            &settings.show_whip_shortcut,
            SHOW_WHIP_SHORTCUT,
            &language,
        )?,
        send_prompt_shortcut: normalize_shortcut(
            &settings.send_prompt_shortcut,
            SEND_PROMPT_SHORTCUT,
            &language,
        )?,
        hide_whip_shortcut: normalize_shortcut(
            &settings.hide_whip_shortcut,
            HIDE_WHIP_SHORTCUT,
            &language,
        )?,
        language,
        crack_send_mode: normalize_crack_send_mode(&settings.crack_send_mode),
        targets,
    };

    let shortcuts = [
        normalized.show_whip_shortcut.as_str(),
        normalized.send_prompt_shortcut.as_str(),
        normalized.hide_whip_shortcut.as_str(),
    ];

    for i in 0..shortcuts.len() {
        for j in (i + 1)..shortcuts.len() {
            if shortcuts[i] == shortcuts[j] {
                return Err(if is_english(&normalized.language) {
                    "Show, send, and hide shortcuts cannot use the same key combination"
                        .to_string()
                } else {
                    "呼出、发送、退出三个快捷键不能设置成同一个组合键".to_string()
                });
            }
        }
    }

    Ok(normalized)
}

struct LocalizedTexts<'a> {
    show_whip: &'a str,
    settings: &'a str,
    quit: &'a str,
    app_menu: &'a str,
    edit_menu: &'a str,
    tray_tooltip: &'a str,
    settings_title: &'a str,
}

fn localized_texts(language: &str) -> LocalizedTexts<'static> {
    if language == LANGUAGE_EN_US {
        LocalizedTexts {
            show_whip: "Show Whip",
            settings: "Settings...",
            quit: "Quit",
            app_menu: "OpenBad",
            edit_menu: "Edit",
            tray_tooltip: "OpenBad - whip companion",
            settings_title: "OpenBad Settings",
        }
    } else {
        LocalizedTexts {
            show_whip: "呼出鞭子",
            settings: "设置...",
            quit: "退出",
            app_menu: "OpenBad",
            edit_menu: "编辑",
            tray_tooltip: "OpenBad - 鼓劲鞭子",
            settings_title: "OpenBad 设置",
        }
    }
}

fn resolve_target_box<'a>(
    targets: &'a [TargetBox],
    payload: CrackPayload,
    width: f64,
    height: f64,
) -> Option<&'a TargetBox> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let px = clamp_f64(payload.x / width * 100.0, 0.0, 100.0);
    let py = clamp_f64(payload.y / height * 100.0, 0.0, 100.0);

    targets.iter().find(|target| {
        px >= target.x
            && px <= target.x + target.width
            && py >= target.y
            && py <= target.y + target.height
    })
}

fn resolve_whip_action(
    settings: &PromptSettings,
    payload: CrackPayload,
    origin_x: f64,
    origin_y: f64,
    width: f64,
    height: f64,
) -> CrackAction {
    let fallback_text = normalize_prompt_text(&settings.prompt_text);

    if settings.crack_send_mode == CRACK_SEND_MODE_SELECTED_INPUT {
        return CrackAction::SendToPreviousSelection {
            text: fallback_text,
        };
    }

    if let Some(target) = resolve_target_box(&settings.targets, payload, width, height) {
        let center_x = ((target.x + target.width / 2.0) / 100.0) * width;
        let center_y = ((target.y + target.height / 2.0) / 100.0) * height;
        let prompt_text = if target.prompt_text.trim().is_empty() {
            fallback_text
        } else {
            target.prompt_text.trim().to_string()
        };

        return CrackAction::ClickAtPoint {
            point: GlobalPoint {
                x: origin_x + center_x,
                y: origin_y + center_y,
            },
            text: prompt_text,
        };
    }

    CrackAction::ClickAtPoint {
        point: GlobalPoint {
            x: origin_x + payload.x,
            y: origin_y + payload.y,
        },
        text: fallback_text,
    }
}

fn settings_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join(SETTINGS_FILE_NAME))
}

fn load_prompt_settings(app: &AppHandle) -> PromptSettings {
    let Ok(path) = settings_file_path(app) else {
        return PromptSettings::default();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return PromptSettings::default();
    };
    serde_json::from_str(&contents)
        .ok()
        .and_then(|settings| normalize_prompt_settings(settings).ok())
        .unwrap_or_default()
}

fn save_prompt_settings(app: &AppHandle, settings: &PromptSettings) -> Result<(), String> {
    let path = settings_file_path(app)?;
    let json = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn sync_global_shortcuts(
    app: &AppHandle,
    previous: Option<&PromptSettings>,
    updated: &PromptSettings,
) -> Result<(), String> {
    let manager = app.global_shortcut();
    let mut to_unregister = Vec::new();
    let mut to_register = Vec::new();
    let mut unregistered: Vec<String> = Vec::new();
    let mut registered: Vec<String> = Vec::new();

    let changed_shortcuts = [
        (
            previous.map(|settings| settings.show_whip_shortcut.as_str()),
            updated.show_whip_shortcut.as_str(),
        ),
        (
            previous.map(|settings| settings.send_prompt_shortcut.as_str()),
            updated.send_prompt_shortcut.as_str(),
        ),
        (
            previous.map(|settings| settings.hide_whip_shortcut.as_str()),
            updated.hide_whip_shortcut.as_str(),
        ),
    ];

    for (old, new) in changed_shortcuts {
        if old == Some(new) {
            continue;
        }

        if let Some(old) = old {
            if manager.is_registered(old) && !to_unregister.contains(&old) {
                to_unregister.push(old);
            }
        }

        if !to_register.contains(&new) {
            to_register.push(new);
        }
    }

    for shortcut in &to_unregister {
        manager
            .unregister(*shortcut)
            .map_err(|error| format!("无法注销旧快捷键 {shortcut}：{error}"))?;
        unregistered.push((*shortcut).to_string());
    }

    for shortcut in &to_register {
        manager.register(*shortcut).map_err(|error| {
            for shortcut in &registered {
                let _ = manager.unregister(shortcut.as_str());
            }
            for shortcut in &unregistered {
                let _ = manager.register(shortcut.as_str());
            }
            format!("无法注册快捷键 {shortcut}：{error}")
        })?;
        registered.push((*shortcut).to_string());
    }

    Ok(())
}

fn show_overlay(app: &AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };

    capture_previous_app(app);
    let _ = configure_overlay_window(app, &window);
    let _ = window.set_focusable(true);
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.show();
    let _ = window.set_focus();
    let lifecycle = app.state::<Mutex<OverlayLifecycle>>();

    if let Ok(mut lifecycle) = lifecycle.lock() {
        if lifecycle.ready {
            emit_spawn_whip(&window);
        } else {
            lifecycle.pending_spawn = true;
        }
    } else {
        emit_spawn_whip(&window);
    };
}

fn handle_menu_action(app: &AppHandle, id: &MenuId) {
    if id == SHOW_MENU_ID {
        show_overlay(app);
    } else if id == SETTINGS_MENU_ID {
        let _ = show_settings_window(app);
    } else if id == QUIT_MENU_ID {
        app.exit(0);
    }
}

fn toggle_overlay(app: &AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };

    let _ = configure_overlay_window(app, &window);

    match window.is_visible() {
        Ok(true) => {
            let _ = window.emit("drop-whip", ());
        }
        _ => {
            show_overlay(app);
        }
    }
}

#[cfg(target_os = "macos")]
fn capture_previous_app(app: &AppHandle) {
    let workspace = NSWorkspace::sharedWorkspace();
    let current_bundle_id = NSRunningApplication::currentApplication()
        .bundleIdentifier()
        .map(|bundle_id| bundle_id.to_string());
    let previous_bundle_id = workspace
        .frontmostApplication()
        .and_then(|application| application.bundleIdentifier())
        .map(|bundle_id| bundle_id.to_string())
        .filter(|bundle_id| current_bundle_id.as_deref() != Some(bundle_id.as_str()));

    if let Ok(mut previous_app) = app.state::<Mutex<PreviousAppState>>().lock() {
        previous_app.bundle_id = previous_bundle_id;
    }
}

#[cfg(not(target_os = "macos"))]
fn capture_previous_app(_app: &AppHandle) {}

#[cfg(target_os = "macos")]
fn focus_previous_app_now(app: &AppHandle) -> Result<(), String> {
    let bundle_id = app
        .state::<Mutex<PreviousAppState>>()
        .lock()
        .map_err(|_| "failed to lock previous app state".to_string())?
        .bundle_id
        .clone()
        .ok_or_else(|| "no previous app recorded".to_string())?;

    let bundle_id = NSString::from_str(&bundle_id);
    let Some(application) = NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id)
        .lastObject()
    else {
        return Err("previous app is no longer running".to_string());
    };

    application.unhide();

    if application.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows) {
        Ok(())
    } else {
        Err("failed to activate previous app".to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn focus_previous_app_now(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

fn refocus_previous_app(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        if let Err(error) = focus_previous_app_now(&app) {
            if error != "no previous app recorded" {
                log::warn!("failed to refocus previous app: {error}");
            }
        }
    });
}

fn suspend_overlay_for_external_input(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return false;
    };

    let was_visible = window.is_visible().unwrap_or(false);
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.set_focusable(false);
    was_visible
}

fn restore_overlay_after_external_input(app: &AppHandle, refocus_overlay: bool) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };

    let _ = window.set_focusable(true);
    let _ = window.set_ignore_cursor_events(false);

    if refocus_overlay {
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn click_at(point: GlobalPoint) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "failed to create CGEventSource".to_string())?;
    let point = CGPoint::new(point.x, point.y);
    let move_event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left,
    )
    .map_err(|_| "failed to create mouse move event".to_string())?;
    let down_event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        point,
        CGMouseButton::Left,
    )
    .map_err(|_| "failed to create mouse down event".to_string())?;
    let up_event =
        CGEvent::new_mouse_event(source, CGEventType::LeftMouseUp, point, CGMouseButton::Left)
            .map_err(|_| "failed to create mouse up event".to_string())?;

    move_event.post(CGEventTapLocation::HID);
    down_event.post(CGEventTapLocation::HID);
    up_event.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn press_return() -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "failed to create CGEventSource".to_string())?;
    let key_down = CGEvent::new_keyboard_event(source.clone(), KeyCode::RETURN, true)
        .map_err(|_| "failed to create return key down event".to_string())?;
    let key_up = CGEvent::new_keyboard_event(source, KeyCode::RETURN, false)
        .map_err(|_| "failed to create return key up event".to_string())?;

    key_down.post(CGEventTapLocation::HID);
    key_up.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn type_text(text: &str) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "failed to create CGEventSource".to_string())?;
    let key_down = CGEvent::new_keyboard_event(source.clone(), 0, true)
        .map_err(|_| "failed to create text key down event".to_string())?;
    let key_up = CGEvent::new_keyboard_event(source, 0, false)
        .map_err(|_| "failed to create text key up event".to_string())?;

    key_down.set_string(text);
    key_up.set_string(text);
    key_down.post(CGEventTapLocation::HID);
    key_up.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn send_text_and_return(text: &str) -> Result<(), String> {
    type_text(text)?;
    thread::sleep(Duration::from_millis(20));
    press_return()
}

#[cfg(target_os = "macos")]
fn send_prompt_to_previous_selection(app: &AppHandle, text: &str) -> Result<(), String> {
    let should_restore_overlay = suspend_overlay_for_external_input(app);
    let result = (|| {
        focus_previous_app_now(app)?;
        thread::sleep(Duration::from_millis(75));
        send_text_and_return(text)?;
        thread::sleep(Duration::from_millis(45));
        Ok(())
    })();

    if should_restore_overlay {
        restore_overlay_after_external_input(app, true);
    }

    result
}

#[cfg(target_os = "macos")]
fn send_macro_to_point(app: &AppHandle, point: GlobalPoint, text: &str) -> Result<(), String> {
    let should_restore_overlay = suspend_overlay_for_external_input(app);
    let result = (|| {
        click_at(point)?;
        thread::sleep(Duration::from_millis(55));
        send_text_and_return(text)?;
        thread::sleep(Duration::from_millis(45));
        Ok(())
    })();

    if should_restore_overlay {
        restore_overlay_after_external_input(app, true);
    }

    result
}

#[cfg(not(target_os = "macos"))]
fn send_macro_to_point(_app: &AppHandle, _point: GlobalPoint, _text: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn send_prompt_to_previous_selection(_app: &AppHandle, _text: &str) -> Result<(), String> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(OverlayLifecycle::default()))
        .manage(Mutex::new(PreviousAppState::default()))
        .invoke_handler(tauri::generate_handler![
            whip_crack,
            get_prompt_settings,
            set_prompt_text,
            hide_overlay,
            overlay_ready,
            set_overlay_passthrough,
            send_prompt_to_focused,
            set_prompt_settings
        ])
        .setup(|app| {
            let settings = load_prompt_settings(app.handle());
            app.manage(Mutex::new(settings.clone()));
            #[cfg(target_os = "macos")]
            {
                let _ = app.set_activation_policy(ActivationPolicy::Accessory);
                let _ = app.set_dock_visibility(false);
            }
            create_overlay_window(app.handle())?;
            build_tray(app.handle(), &settings.language)?;
            build_app_menu(app.handle(), &settings.language)?;
            app.handle().plugin(
                GlobalShortcutBuilder::new()
                    .with_handler(|app, shortcut, event| {
                        if event.state != ShortcutState::Pressed {
                            return;
                        }

                        let settings_state = app.state::<Mutex<PromptSettings>>();
                        let Ok(settings) = settings_state.lock() else {
                            return;
                        };
                        let show_whip_id = Shortcut::from_str(&settings.show_whip_shortcut)
                            .ok()
                            .map(|shortcut| shortcut.id());
                        let send_prompt_id = Shortcut::from_str(&settings.send_prompt_shortcut)
                            .ok()
                            .map(|shortcut| shortcut.id());
                        let hide_whip_id = Shortcut::from_str(&settings.hide_whip_shortcut)
                            .ok()
                            .map(|shortcut| shortcut.id());
                        let prompt_text = normalize_prompt_text(&settings.prompt_text);
                        drop(settings);

                        if Some(shortcut.id()) == show_whip_id {
                            show_overlay(app);
                        } else if Some(shortcut.id()) == hide_whip_id {
                            hide_overlay_for_app(app);
                        } else if Some(shortcut.id()) == send_prompt_id {
                            if let Err(error) =
                                send_prompt_to_previous_selection(app, &prompt_text)
                            {
                                log::warn!("failed to send prompt from shortcut: {error}");
                            }
                        }
                    })
                    .build(),
            )?;
            if let Err(error) = sync_global_shortcuts(app.handle(), None, &settings) {
                log::warn!("failed to register default shortcuts: {error}");
            }
            app.on_menu_event(|app, event| {
                handle_menu_action(app, event.id());
            });
            show_overlay(app.handle());

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .enable_macos_default_menu(false)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
