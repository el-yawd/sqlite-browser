use crate::file_manager::{FileManager, FileManagerEvent};
use crate::models::{DatabaseInfo, PageInfo};

use crate::ui::entities::{
    FileDialogManager, FileOpenError, FileOpened, PageGrid, PageSelected, PageSidebar,
};
use crate::ui::status_manager::{StatusAction, StatusManager};
use anyhow::Result;
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, IntoElement, ParentElement, Render, Subscription,
    Task, Window, actions, div, impl_actions, prelude::*, px,
};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

actions![sqlite_browser, [OpenFile, RefreshDatabase]];

#[derive(Clone, Default, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct SelectPage {
    pub page_number: u32,
}

impl_actions!(sqlite_browser, [SelectPage]);

pub struct SqliteBrowser {
    file_manager: FileManager,
    pub database_info: Option<Arc<DatabaseInfo>>,
    focus_handle: FocusHandle,
    status_manager: StatusManager,

    // Entity handles
    file_dialog: Entity<FileDialogManager>,
    page_grid: Entity<PageGrid>,
    page_sidebar: Entity<PageSidebar>,

    // Subscriptions
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<FileManagerEvent> for SqliteBrowser {}

impl SqliteBrowser {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Create entities
        let file_dialog = cx.new(|_cx| FileDialogManager::new());
        let page_grid = cx.new(|_cx| PageGrid::new(Arc::new(BTreeMap::new())));
        let page_sidebar = cx.new(|_cx| PageSidebar::new());

        let mut browser = Self {
            file_manager: FileManager::new(),
            database_info: None,
            focus_handle: cx.focus_handle(),
            status_manager: StatusManager::new(),
            file_dialog: file_dialog.clone(),
            page_grid: page_grid.clone(),
            page_sidebar: page_sidebar.clone(),
            _subscriptions: Vec::new(),
        };

        // Set up subscriptions between entities
        let file_opened_subscription = cx.subscribe(&file_dialog, {
            move |this, _entity, event: &FileOpened, cx| {
                this.handle_file_opened(event.path.clone(), event.database_info.clone(), cx);
            }
        });

        let file_error_subscription = cx.subscribe(&file_dialog, {
            move |this, _entity, event: &FileOpenError, cx| {
                this.status_manager.show_error(
                    format!("Failed to open {}: {}", event.path.display(), event.error),
                    cx,
                );
            }
        });

        // Subscribe to FileManagerEvent emissions from this entity
        let file_manager_subscription = cx.subscribe(&cx.entity(), {
            move |this, _entity, event: &FileManagerEvent, cx| {
                this.handle_file_manager_event(event, cx);
            }
        });

        let page_selected_subscription = cx.subscribe(&page_grid, {
            move |this, _entity, event: &PageSelected, cx| {
                this.handle_page_selected(event.page_number, cx);
            }
        });

        browser._subscriptions.extend([
            file_opened_subscription,
            file_error_subscription,
            file_manager_subscription,
            page_selected_subscription,
        ]);

        browser
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.file_dialog
            .update(cx, |dialog, cx| dialog.open_file(path, cx))
    }

    pub fn close_current_file(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.file_manager.current_file().map(|p| p.to_path_buf()) {
            self.file_manager.stop_watching(cx);
            self.file_manager.set_current_file(None);
            self.database_info = None;

            // Clear entities
            self.page_grid.update(cx, |grid, cx| {
                grid.update_pages(Arc::new(BTreeMap::new()), cx);
            });
            self.page_sidebar.update(cx, |sidebar, cx| {
                sidebar.update_data(None, None, cx);
            });

            self.status_manager.clear_all(cx);
            cx.emit(FileManagerEvent::FileDeleted(path));
            cx.notify();
        }
    }

    fn handle_file_opened(
        &mut self,
        path: PathBuf,
        database_info: Arc<DatabaseInfo>,
        cx: &mut Context<Self>,
    ) {
        self.file_manager.set_current_file(Some(path.clone()));
        self.database_info = Some(database_info.clone());

        // Update entities with new data
        self.page_grid.update(cx, |grid, cx| {
            grid.update_pages(database_info.pages.clone(), cx);
        });

        self.page_sidebar.update(cx, |sidebar, cx| {
            sidebar.update_data(None, Some(database_info.clone()), cx);
        });

        // Start watching the file
        if let Err(e) = self.file_manager.start_watching(&path, cx) {
            eprintln!("Failed to start watching file: {}", e);
            self.status_manager.show_warning(
                format!(
                    "Could not start watching {} for changes: {}",
                    path.display(),
                    e
                ),
                cx,
            );
        }

        self.status_manager
            .show_success(format!("Opened {}", path.display()), cx);
        cx.notify();
    }

    fn handle_page_selected(&mut self, page_number: u32, cx: &mut Context<Self>) {
        eprintln!("Page selected {}", page_number);
        self.page_sidebar.update(cx, |sidebar, cx| {
            sidebar.set_selected_page(Some(page_number), cx);
        });
    }

    pub fn current_file_path(&self) -> Option<&std::path::Path> {
        self.file_manager.current_file()
    }

    pub fn is_file_being_watched(&self) -> bool {
        self.file_manager.is_watching()
    }

    pub fn selected_page_info(&self, cx: &Context<Self>) -> Option<PageInfo> {
        let selected_page = self.page_sidebar.read(cx).selected_page?;
        let database_info = self.database_info.as_ref()?;
        Some(database_info.pages.get(&selected_page)?.clone())
    }

    /// Try to open a file or show dialog if path doesn't exist
    pub fn try_open_file_or_dialog(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        self.file_dialog
            .update(cx, |dialog, cx| dialog.try_open_file_or_dialog(path, cx))
    }

    /// Open a file dialog to select a SQLite database file
    pub fn open_file_dialog(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.file_dialog
            .update(cx, |dialog, cx| dialog.open_file_dialog(cx))
    }

    fn handle_file_manager_event(&mut self, event: &FileManagerEvent, cx: &mut Context<Self>) {
        match event {
            FileManagerEvent::FileOpened(path, database_info) => {
                self.database_info = Some(database_info.clone());
                self.status_manager
                    .show_success(format!("Opened {}", path.display()), cx);
            }
            FileManagerEvent::FileModified(path, database_info) => {
                self.database_info = Some(database_info.clone());

                // Update last modification time
                self.file_manager
                    .update_last_modification(std::time::Instant::now());

                // Update entities with new data
                self.page_grid.update(cx, |grid, cx| {
                    grid.update_pages(database_info.pages.clone(), cx);
                });
                self.page_sidebar.update(cx, |sidebar, cx| {
                    sidebar.update_data(sidebar.selected_page, Some(database_info.clone()), cx);
                });

                self.status_manager.show_info(
                    format!("File {} was modified and reloaded", path.display()),
                    cx,
                );
                cx.notify();
            }
            FileManagerEvent::FileDeleted(path) => {
                self.database_info = None;
                self.file_manager.set_current_file(None);

                // Clear entities
                self.page_grid.update(cx, |grid, cx| {
                    grid.update_pages(Arc::new(BTreeMap::new()), cx);
                });
                self.page_sidebar.update(cx, |sidebar, cx| {
                    sidebar.update_data(None, None, cx);
                });

                self.status_manager
                    .show_error(format!("File {} was deleted", path.display()), cx);
                cx.notify();
            }
            FileManagerEvent::ParseError(path, error) => {
                self.status_manager
                    .show_error(format!("Error parsing {}: {}", path.display(), error), cx);
            }
            FileManagerEvent::WatchingStarted(path) => {
                self.status_manager.show_info(
                    format!("Started watching {} for changes", path.display()),
                    cx,
                );
            }
            FileManagerEvent::WatchingStopped(path) => {
                self.status_manager.show_info(
                    format!("Stopped watching {} for changes", path.display()),
                    cx,
                );
            }
            FileManagerEvent::WatchingFailed(path, error) => {
                // Mark the file manager as failed and show error
                self.file_manager.mark_watching_failed();
                self.status_manager.show_error(
                    format!(
                        "File watching failed for {}: {}. Manual refresh available.",
                        path.display(),
                        error
                    ),
                    cx,
                );
            }
            FileManagerEvent::ParseStarted(path) => {
                self.status_manager.show_progress(
                    format!(
                        "Loading {}...",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
                    ),
                    0.0,
                    cx,
                );
            }
            FileManagerEvent::ParseProgress(_path, progress) => {
                self.status_manager.update_progress(*progress, cx);
            }
            FileManagerEvent::ParseCompleted(_path) => {
                self.status_manager.dismiss_message(cx);
            }
            FileManagerEvent::ParseCancelled(path) => {
                self.status_manager.show_info(
                    format!(
                        "Cancelled loading {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("file")
                    ),
                    cx,
                );
            }
        }
    }
}

impl Render for SqliteBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().size_full().bg(gpui::rgb(0x1e1e1e)).child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .h_full()
                .child(self.render_header_with_handlers(cx))
                .when_some(self.status_manager.render(), |this, status_element| {
                    this.child(self.render_status_with_handlers(status_element, cx))
                })
                .child(if self.database_info.is_some() {
                    div()
                        .flex()
                        .flex_1()
                        .child(div().flex_1().child(self.page_grid.clone()))
                        .child(self.page_sidebar.clone())
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .child(self.file_dialog.clone())
                        .into_any_element()
                }),
        )
    }
}

impl SqliteBrowser {
    fn render_status_with_handlers(
        &self,
        status_element: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Add click handlers for dismiss and action buttons
        div().child(status_element).on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _event, _window, cx| {
                // For now, just dismiss the message when clicked anywhere
                // In a real implementation, we'd need to check which element was clicked
                this.status_manager.dismiss_message(cx);
            }),
        )
    }

    fn handle_status_action(&mut self, action: StatusAction, cx: &mut Context<Self>) {
        match action {
            StatusAction::Retry => {
                // Retry the last operation - this would need more context
                // For now, just dismiss the message
                self.status_manager.dismiss_message(cx);
            }
            StatusAction::Dismiss => {
                self.status_manager.dismiss_message(cx);
            }
            StatusAction::OpenFile => {
                self.status_manager.dismiss_message(cx);
                self.open_file_dialog(cx).detach();
            }
            StatusAction::ShowDetails => {
                // Show more details - for now just dismiss
                self.status_manager.dismiss_message(cx);
            }
            StatusAction::Cancel => {
                self.cancel_current_operation(cx);
            }
        }
    }

    pub fn cancel_current_operation(&mut self, cx: &mut Context<Self>) {
        if self.file_manager.is_parsing() {
            self.file_manager.cancel_current_parse();
            self.status_manager
                .show_info("Cancelling operation...".to_string(), cx);
        }
    }

    pub fn is_operation_in_progress(&self) -> bool {
        self.file_manager.is_parsing()
    }

    fn render_header_with_handlers(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .bg(gpui::rgb(0x2d2d2d))
            .border_b_1()
            .border_color(gpui::rgb(0x3e3e3e))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(gpui::rgb(0xffffff))
                            .child("SQLite Browser"),
                    )
                    .when_some(self.current_file_path(), |this, path| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0xcccccc))
                                .child(format!(
                                    "- {}",
                                    path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown")
                                )),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .bg(gpui::rgb(0x2563eb))
                            .hover(|this| this.bg(gpui::rgb(0x1d4ed8)))
                            .rounded_md()
                            .cursor_pointer()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _event, _window, cx| {
                                    this.open_file_dialog(cx).detach();
                                }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(gpui::rgb(0xffffff))
                                    .child("Open File"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0xaaaaaa))
                            .child(format!(
                                "Pages: {}",
                                self.database_info
                                    .as_ref()
                                    .map_or(0, |info| info.page_count())
                            )),
                    )
                    .when(self.is_file_being_watched(), |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(div().size(px(8.0)).rounded_full().bg(gpui::rgb(0x4CAF50)))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x4CAF50))
                                        .child("Watching"),
                                ),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::models::{DatabaseHeader, PageType};
    use gpui::{Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, point, px};

    fn database_with_page_count(page_count: u32) -> Arc<DatabaseInfo> {
        let header = DatabaseHeader {
            magic: *b"SQLite format 3\0",
            page_size: 4096,
            file_format_write_version: 1,
            file_format_read_version: 1,
            reserved_space: 0,
            max_embedded_payload_fraction: 64,
            min_embedded_payload_fraction: 32,
            leaf_payload_fraction: 32,
            file_change_counter: 1,
            database_size_pages: page_count,
            first_freelist_trunk_page: 0,
            total_freelist_pages: 0,
            schema_cookie: 1,
            schema_format_number: 4,
            default_page_cache_size: 0,
            largest_root_btree_page: 0,
            text_encoding: 1,
            user_version: 0,
            incremental_vacuum_mode: 0,
            application_id: 0,
            version_valid_for: 1,
            sqlite_version_number: 3_045_001,
        };
        let pages = (1..=page_count)
            .map(|page_number| {
                let page_type = if page_number % 2 == 0 {
                    PageType::IndexBTreeLeaf
                } else {
                    PageType::TableBTreeLeaf
                };
                (
                    page_number,
                    PageInfo::new(page_number, page_type, 3, 2048, 0, None),
                )
            })
            .collect();

        Arc::new(DatabaseInfo::new(
            header,
            Arc::new(pages),
            u64::from(page_count) * 4096,
        ))
    }

    fn install_database(
        browser: &Entity<SqliteBrowser>,
        database: Arc<DatabaseInfo>,
        cx: &mut TestAppContext,
    ) {
        browser.update(cx, |browser, cx| {
            browser.database_info = Some(database.clone());
            browser.page_grid.update(cx, |grid, cx| {
                grid.update_pages(database.pages.clone(), cx);
            });
            browser.page_sidebar.update(cx, |sidebar, cx| {
                sidebar.update_data(None, Some(database), cx);
            });
            cx.notify();
        });
    }

    #[gpui::test]
    fn clicking_a_rendered_page_updates_grid_and_sidebar(cx: &mut TestAppContext) {
        let (browser, cx) = cx.add_window_view(|_window, cx| SqliteBrowser::new(cx));
        install_database(&browser, database_with_page_count(2), cx);
        cx.update(|window, _cx| window.refresh());

        assert!(cx.debug_bounds("sidebar-empty").is_some());
        let page_bounds = cx
            .debug_bounds("page-2")
            .expect("page 2 should be rendered");
        cx.simulate_click(page_bounds.center(), Modifiers::default());

        let (page_grid, page_sidebar) = cx.read(|cx| {
            let browser = browser.read(cx);
            (browser.page_grid.clone(), browser.page_sidebar.clone())
        });
        assert_eq!(
            cx.read(|cx| page_grid.read(cx).get_selected_page()),
            Some(2)
        );
        assert_eq!(cx.read(|cx| page_sidebar.read(cx).selected_page), Some(2));
        assert!(cx.debug_bounds("sidebar-loaded").is_some());
    }

    #[gpui::test]
    fn clicking_an_error_status_message_dismisses_it(cx: &mut TestAppContext) {
        let (browser, cx) = cx.add_window_view(|_window, cx| SqliteBrowser::new(cx));
        browser.update(cx, |browser, cx| {
            browser
                .status_manager
                .show_error("Invalid database".to_string(), cx);
        });
        cx.update(|window, _cx| window.refresh());

        let status_bounds = cx
            .debug_bounds("status-message")
            .expect("error status should be rendered");
        cx.simulate_click(status_bounds.center(), Modifiers::default());

        assert!(cx.read(|cx| browser.read(cx).status_manager.current_message().is_none()));
    }

    #[gpui::test]
    #[ignore = "known issue: clicking a non-dismissible progress message dismisses it"]
    fn progress_status_cannot_be_dismissed_by_clicking_its_body(cx: &mut TestAppContext) {
        let (browser, cx) = cx.add_window_view(|_window, cx| SqliteBrowser::new(cx));
        browser.update(cx, |browser, cx| {
            browser
                .status_manager
                .show_progress("Loading database".to_string(), 0.5, cx);
        });
        cx.update(|window, _cx| window.refresh());

        let status_bounds = cx
            .debug_bounds("status-message")
            .expect("progress status should be rendered");
        cx.simulate_click(status_bounds.center(), Modifiers::default());

        assert!(cx.read(|cx| browser.read(cx).status_manager.current_message().is_some()));
    }

    #[gpui::test]
    #[ignore = "known issue: success and info messages never auto-dismiss"]
    fn success_status_auto_dismisses_after_three_seconds(cx: &mut TestAppContext) {
        let (browser, cx) = cx.add_window_view(|_window, cx| SqliteBrowser::new(cx));
        browser.update(cx, |browser, cx| {
            browser
                .status_manager
                .show_success("Database opened".to_string(), cx);
        });

        cx.executor()
            .advance_clock(std::time::Duration::from_secs(4));
        cx.run_until_parked();

        assert!(cx.read(|cx| browser.read(cx).status_manager.current_message().is_none()));
    }

    #[gpui::test]
    #[ignore = "known issue: the page grid has no keyboard navigation"]
    fn right_arrow_selects_the_first_page(cx: &mut TestAppContext) {
        let (browser, cx) = cx.add_window_view(|_window, cx| SqliteBrowser::new(cx));
        install_database(&browser, database_with_page_count(2), cx);
        cx.update(|window, _cx| window.refresh());

        cx.simulate_keystrokes("right");

        let page_grid = cx.read(|cx| browser.read(cx).page_grid.clone());
        assert_eq!(
            cx.read(|cx| page_grid.read(cx).get_selected_page()),
            Some(1)
        );
    }

    #[gpui::test]
    #[ignore = "known issue: overflow_hidden clips the page grid instead of scrolling it"]
    fn scrolling_moves_later_pages_into_view(cx: &mut TestAppContext) {
        let (browser, cx) = cx.add_window_view(|_window, cx| SqliteBrowser::new(cx));
        install_database(&browser, database_with_page_count(200), cx);
        cx.update(|window, _cx| window.refresh());

        let container = cx
            .debug_bounds("page-grid-container")
            .expect("page grid should be rendered");
        let before = cx
            .debug_bounds("page-200")
            .expect("last page should be laid out");
        cx.simulate_event(ScrollWheelEvent {
            position: container.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-5000.0))),
            ..Default::default()
        });
        let after = cx
            .debug_bounds("page-200")
            .expect("last page should remain laid out");

        assert!(
            after.origin.y < before.origin.y,
            "scrolling should move later pages toward the visible grid"
        );
    }
}
