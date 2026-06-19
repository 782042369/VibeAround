//! System tray for the environment configurator.

use serde::{Deserialize, Serialize};
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItemBuilder},
    tray::{TrayIcon, TrayIconBuilder},
    App, AppHandle, Manager, Runtime,
};

const TRAY_ID: &str = "main";
const MAIN_WINDOW_LABEL: &str = "main";
const MENU_SHOW_WINDOW: &str = "show_window";
const MENU_QUIT: &str = "quit";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UiLocale {
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

impl UiLocale {
    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en_us" => Some(Self::En),
            "zh" | "zh-cn" | "zh_cn" | "zh-hans" | "zh_hans" => Some(Self::ZhCn),
            _ => None,
        }
    }

    fn text(self, key: TrayText) -> &'static str {
        match self {
            Self::En => key.en(),
            Self::ZhCn => match key {
                TrayText::ShowWindow => "显示窗口",
                TrayText::Quit => "退出",
            },
        }
    }
}

#[derive(Clone, Copy)]
enum TrayText {
    ShowWindow,
    Quit,
}

impl TrayText {
    fn en(self) -> &'static str {
        match self {
            Self::ShowWindow => "Show Window",
            Self::Quit => "Quit",
        }
    }
}

pub fn setup<R: Runtime>(app: &App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let menu = build_menu(&app_handle)?;

    const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/32x32.png");
    let icon = Image::from_bytes(TRAY_ICON_PNG)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip("VibeWbz")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            MENU_SHOW_WINDOW => {
                if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = w.unminimize();
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    build_menu_for_locale(app, UiLocale::En)
}

fn build_menu_for_locale<R: Runtime>(
    app: &AppHandle<R>,
    locale: UiLocale,
) -> tauri::Result<Menu<R>> {
    let show_item =
        MenuItemBuilder::with_id(MENU_SHOW_WINDOW, locale.text(TrayText::ShowWindow)).build(app)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, locale.text(TrayText::Quit)).build(app)?;

    MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&quit_item)
        .build()
}

pub(crate) fn set_ui_locale<R: Runtime>(app: &AppHandle<R>, locale: &str) -> Result<(), String> {
    let locale =
        UiLocale::from_str(locale).ok_or_else(|| format!("unsupported locale: {locale}"))?;
    rebuild_menu(app, locale).map_err(|e| e.to_string())
}

fn rebuild_menu<R: Runtime>(app: &AppHandle<R>, locale: UiLocale) -> tauri::Result<()> {
    let menu = build_menu_for_locale(app, locale)?;
    if let Some(tray) = tray_icon(app) {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

fn tray_icon<R: Runtime>(app: &AppHandle<R>) -> Option<TrayIcon<R>> {
    app.tray_by_id(TRAY_ID)
}
