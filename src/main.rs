use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, actions, prelude::*, px, size};
use std::path::PathBuf;

mod models;
mod parser;
mod file_manager;
mod ui;

use ui::SqliteBrowser;

actions!(sqlite_browser, [OpenFile, SelectPage, RefreshDatabase]);

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

                    // Register action handlers
                    SqliteBrowser::register_actions(cx);

                    // Try to open file from command line argument
                    if let Some(path) = std::env::args().nth(1) {
                        let path = PathBuf::from(path);
                        if path.exists() {
                            browser.open_file(path, cx).detach();
                        } else {
                            eprintln!("Warning: File '{}' does not exist", path.display());
                        }
                    }

                    browser
                })
            },
        )
        .unwrap();
    });
}