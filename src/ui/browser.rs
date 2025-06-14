use crate::file_manager::{FileManager, FileManagerEvent};
use crate::models::{DatabaseInfo, PageInfo};
use crate::ui::components;
use anyhow::Result;
use gpui::{
    Context, EventEmitter, FocusHandle, IntoElement, ParentElement, Render, Task, Window, actions,
    div, impl_actions, prelude::*, px,
};
use rfd::FileDialog;
use std::path::PathBuf;

actions![sqlite_browser, [OpenFile, RefreshDatabase]];

#[derive(Clone, Default, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct SelectPage {
    pub page_number: u32,
}

impl_actions!(sqlite_browser, [SelectPage]);

pub struct SqliteBrowser {
    file_manager: FileManager,
    database_info: Option<DatabaseInfo>,
    selected_page: Option<u32>,
    focus_handle: FocusHandle,
    status_message: Option<(String, bool)>, // (message, is_error)
}

impl EventEmitter<FileManagerEvent> for SqliteBrowser {}

impl SqliteBrowser {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            file_manager: FileManager::new(),
            database_info: None,
            selected_page: None,
            focus_handle: cx.focus_handle(),
            status_message: None,
        }
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.clear_status_message(cx);

        // Reset state when opening new file
        self.database_info = None;
        self.selected_page = None;
        cx.notify();

        let parse_task = self.file_manager.open_file(path.clone(), cx);

        cx.spawn(async move |entity, cx| {
            match parse_task.await {
                Ok(database_info) => {
                    entity.update(cx, |this, cx| {
                        this.file_manager.set_current_file(Some(path.clone()));
                        this.database_info = Some(database_info.clone());

                        // Start watching the file
                        if let Err(e) = this.file_manager.start_watching(&path, cx) {
                            eprintln!("Failed to start watching file: {}", e);
                        }

                        cx.emit(FileManagerEvent::FileOpened(path, database_info));
                    })?;
                    Ok(())
                }
                Err(e) => {
                    entity.update(cx, |this, cx| {
                        this.set_status_message(
                            format!("Failed to open {}: {}", path.display(), e),
                            true,
                            cx,
                        );
                        cx.emit(FileManagerEvent::ParseError(path, e.to_string()));
                    })?;
                    Err(e)
                }
            }
        })
    }

    pub fn refresh_database(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.clear_status_message(cx);

        let refresh_task = self.file_manager.refresh_current_file(cx);

        cx.spawn(async move |entity, cx| {
            match refresh_task.await {
                Ok(database_info) => {
                    entity.update(cx, |this, cx| {
                        // Preserve selected page if it still exists
                        if let Some(selected) = this.selected_page {
                            if !database_info
                                .pages
                                .iter()
                                .any(|p| p.page_number == selected)
                            {
                                this.selected_page = None;
                            }
                        }

                        this.database_info = Some(database_info.clone());
                        this.set_status_message("Database refreshed".to_string(), false, cx);

                        if let Some(path) = this.file_manager.current_file() {
                            cx.emit(FileManagerEvent::FileModified(
                                path.to_path_buf(),
                                database_info,
                            ));
                        }
                    })?;
                    Ok(())
                }
                Err(e) => {
                    entity.update(cx, |this, cx| {
                        this.set_status_message(
                            format!("Failed to refresh database: {}", e),
                            true,
                            cx,
                        );
                    })?;
                    Err(e)
                }
            }
        })
    }

    pub fn close_current_file(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.file_manager.current_file().map(|p| p.to_path_buf()) {
            self.file_manager.stop_watching();
            self.file_manager.set_current_file(None);
            self.database_info = None;
            self.selected_page = None;
            self.clear_status_message(cx);
            cx.emit(FileManagerEvent::FileDeleted(path));
            cx.notify();
        }
    }

    fn select_page(&mut self, page_number: u32, cx: &mut Context<Self>) {
        // Validate that the page exists
        if let Some(ref db_info) = self.database_info {
            if let Some(page) = db_info.pages.iter().find(|p| p.page_number == page_number) {
                self.selected_page = Some(page_number);
                self.set_status_message(
                    format!("Selected page {} ({})", page_number, page.page_type.name()),
                    false,
                    cx,
                );
                cx.notify();
            } else {
                self.set_status_message(format!("Page {} not found", page_number), true, cx);
            }
        } else {
            self.set_status_message("No database loaded".to_string(), true, cx);
        }
    }

    fn set_status_message(&mut self, message: String, is_error: bool, cx: &mut Context<Self>) {
        self.status_message = Some((message, is_error));
        cx.notify();
    }

    fn clear_status_message(&mut self, cx: &mut Context<Self>) {
        if self.status_message.is_some() {
            self.status_message = None;
            cx.notify();
        }
    }

    pub fn current_file_path(&self) -> Option<&std::path::Path> {
        self.file_manager.current_file()
    }

    pub fn is_file_being_watched(&self) -> bool {
        self.file_manager.is_watching()
    }

    pub fn database_info(&self) -> Option<&DatabaseInfo> {
        self.database_info.as_ref()
    }

    pub fn selected_page_info(&self) -> Option<&PageInfo> {
        if let (Some(selected), Some(db_info)) = (self.selected_page, &self.database_info) {
            db_info.get_page(selected)
        } else {
            None
        }
    }

    /// Programmatically select a page by its number
    pub fn select_page_by_number(&mut self, page_number: u32, cx: &mut Context<Self>) -> bool {
        let action = SelectPage { page_number };
        self.handle_select_page(&action, cx);
        self.selected_page == Some(page_number)
    }

    /// Get the currently selected page number
    pub fn selected_page_number(&self) -> Option<u32> {
        self.selected_page
    }

    /// Open a file dialog to select a SQLite database file
    pub fn open_file_dialog(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        cx.spawn(async move |entity, cx| {
            // Use async file dialog to avoid blocking the UI
            match FileDialog::new()
                .add_filter("All Files", &["*"])
                .set_title("Open SQLite Database")
                .pick_file()
            {
                Some(path) => {
                    entity.update(cx, |this, cx| {
                        this.open_file(path, cx).detach();
                    })?;
                    Ok(())
                }
                None => {
                    // User cancelled the dialog
                    entity.update(cx, |this, cx| {
                        this.set_status_message("File selection cancelled".to_string(), false, cx);
                    })?;
                    Ok(())
                }
            }
        })
    }

    /// Try to open a file, and if it fails, open a file dialog
    pub fn try_open_file_or_dialog(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        if path.exists() {
            self.open_file(path, cx)
        } else {
            self.set_status_message(
                format!("File '{}' not found. Please select a file.", path.display()),
                true,
                cx,
            );
            self.open_file_dialog(cx)
        }
    }

    fn handle_file_manager_event(&mut self, event: &FileManagerEvent, cx: &mut Context<Self>) {
        match event {
            FileManagerEvent::FileOpened(path, database_info) => {
                self.database_info = Some(database_info.clone());
                self.set_status_message(format!("Opened {}", path.display()), false, cx);
            }
            FileManagerEvent::FileModified(path, database_info) => {
                // Preserve selected page if it still exists
                if let Some(selected) = self.selected_page {
                    if !database_info
                        .pages
                        .iter()
                        .any(|p| p.page_number == selected)
                    {
                        self.selected_page = None;
                    }
                }

                self.database_info = Some(database_info.clone());
                self.set_status_message(
                    format!("File {} was modified and reloaded", path.display()),
                    false,
                    cx,
                );
            }
            FileManagerEvent::FileDeleted(path) => {
                self.database_info = None;
                self.selected_page = None;
                self.file_manager.set_current_file(None);
                self.set_status_message(format!("File {} was deleted", path.display()), true, cx);
            }
            FileManagerEvent::ParseError(path, error) => {
                self.set_status_message(
                    format!("Error parsing {}: {}", path.display(), error),
                    true,
                    cx,
                );
            }
        }
    }
}

impl Render for SqliteBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Set up event handling for file manager events
        cx.subscribe(
            &cx.entity(),
            |this, _entity, event: &FileManagerEvent, cx| {
                this.handle_file_manager_event(event, cx);
            },
        )
        .detach();

        div().flex().size_full().bg(gpui::rgb(0x1e1e1e)).child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .h_full()
                .child(self.render_header_with_handlers(cx))
                .when_some(self.status_message.as_ref(), |this, (message, is_error)| {
                    this.child(components::render_status_message(message, *is_error))
                })
                .child(if let Some(ref db_info) = self.database_info {
                    div()
                        .flex()
                        .flex_1()
                        .child(self.render_page_grid_with_handlers(&db_info.pages, cx))
                        .child(components::render_sidebar(
                            self.selected_page,
                            &db_info.pages,
                            Some(db_info),
                        ))
                        .into_any_element()
                } else {
                    self.render_empty_state_with_handlers(cx).into_any_element()
                }),
        )
    }
}

impl SqliteBrowser {
    fn render_page_grid_with_handlers(
        &self,
        pages: &[PageInfo],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut page_grid = div().flex().flex_wrap().gap_2();

        for page in pages {
            let page_number = page.page_number;
            let is_selected = self.selected_page == Some(page_number);

            page_grid = page_grid.child(
                div()
                    .size(px(80.0))
                    .bg(page.page_type.color())
                    .when(is_selected, |this| {
                        this.border_2().border_color(gpui::rgb(0xffffff))
                    })
                    .when(!is_selected, |this| {
                        this.border_1().border_color(gpui::rgb(0x555555))
                    })
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|this| this.opacity(0.7))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            let action = SelectPage { page_number };
                            this.handle_select_page(&action, cx);
                        }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(gpui::rgb(0xffffff))
                            .child(format!("{}", page.page_number)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(0xffffff))
                            .opacity(0.8)
                            .child(page.page_type.short_name()),
                    ),
            );
        }

        div().flex().flex_1().flex_col().p_4().child(page_grid)
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

    fn render_empty_state_with_handlers(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .text_color(gpui::rgb(0xaaaaaa))
                    .child("No database loaded"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(gpui::rgb(0x888888))
                    .child("Open a SQLite database file to get started"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_6()
                    .py_3()
                    .bg(gpui::rgb(0x2563eb))
                    .hover(|this| this.bg(gpui::rgb(0x1d4ed8)))
                    .rounded_lg()
                    .cursor_pointer()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.open_file_dialog(cx).detach();
                        }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(gpui::rgb(0xffffff))
                            .child("Open File"),
                    ),
            )
    }

    pub fn register_actions(_cx: &mut Context<Self>) {
        // Action registration in GPUI is complex - for now we use direct method calls
        // This keeps the action handlers available for future proper integration
    }

    fn handle_open_file(&mut self, _action: &OpenFile, cx: &mut Context<Self>) {
        // Open file dialog to select a database file
        self.open_file_dialog(cx).detach();
    }

    pub fn handle_select_page(&mut self, action: &SelectPage, cx: &mut Context<Self>) {
        println!("Handling select page action");
        self.select_page(action.page_number, cx);
    }

    fn handle_refresh_database(&mut self, _action: &RefreshDatabase, cx: &mut Context<Self>) {
        self.refresh_database(cx).detach();
    }
}
