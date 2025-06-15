use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, actions, prelude::*, px, size};
use std::path::PathBuf;

mod file_manager;
mod models;
mod parser;
mod ui;

use ui::SqliteBrowser;

actions!(sqlite_browser, [OpenFile, RefreshDatabase]);

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    let mut browser = SqliteBrowser::new(cx);

                    // Try to open file from command line argument, or show file dialog
                    if let Some(path) = std::env::args().nth(1) {
                        let path = PathBuf::from(path);
                        browser.try_open_file_or_dialog(path, cx).detach();
                    } else {
                        // No file provided, show file dialog
                        browser.open_file_dialog(cx).detach();
                    }

                    browser
                })
            },
        )
        .unwrap();
    });
}
