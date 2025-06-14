use anyhow::Result;
use gpui::{
    Context, EventEmitter, FocusHandle, IntoElement, ParentElement, Render, Task, Window,
    actions, div, prelude::*, px,
};
use std::path::PathBuf;
use crate::file_manager::{FileManager, FileManagerEvent};
use crate::models::{DatabaseInfo, PageInfo};
use crate::ui::components;

actions![sqlite_browser, [OpenFile, SelectPage, RefreshDatabase]];

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
                        this.set_status_message(
                            format!("Opened {}", path.display()),
                            false,
                            cx,
                        );
                        
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
                            if !database_info.pages.iter().any(|p| p.page_number == selected) {
                                this.selected_page = None;
                            }
                        }
                        
                        this.database_info = Some(database_info.clone());
                        this.set_status_message("Database refreshed".to_string(), false, cx);
                        
                        if let Some(path) = this.file_manager.current_file() {
                            cx.emit(FileManagerEvent::FileModified(path.to_path_buf(), database_info));
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
            if db_info.pages.iter().any(|p| p.page_number == page_number) {
                self.selected_page = Some(page_number);
                cx.notify();
            }
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

    fn handle_file_manager_event(&mut self, event: &FileManagerEvent, cx: &mut Context<Self>) {
        match event {
            FileManagerEvent::FileOpened(path, database_info) => {
                self.database_info = Some(database_info.clone());
                self.set_status_message(
                    format!("Opened {}", path.display()),
                    false,
                    cx,
                );
            }
            FileManagerEvent::FileModified(path, database_info) => {
                // Preserve selected page if it still exists
                if let Some(selected) = self.selected_page {
                    if !database_info.pages.iter().any(|p| p.page_number == selected) {
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
                self.set_status_message(
                    format!("File {} was deleted", path.display()),
                    true,
                    cx,
                );
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
        cx.subscribe(&cx.entity(), |this, _entity, event: &FileManagerEvent, cx| {
            this.handle_file_manager_event(event, cx);
        }).detach();

        div()
            .flex()
            .size_full()
            .bg(gpui::rgb(0x1e1e1e))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .child(components::render_header(
                        self.current_file_path(),
                        self.database_info.as_ref().map_or(0, |info| info.page_count()),
                        self.is_file_being_watched(),
                    ))
                    .when_some(self.status_message.as_ref(), |this, (message, is_error)| {
                        this.child(components::render_status_message(message, *is_error))
                    })
                    .child(
                        if let Some(ref db_info) = self.database_info {
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
                            components::render_empty_state().into_any_element()
                        }
                    )
            )
    }
}

impl SqliteBrowser {
    fn render_page_grid_with_handlers(&self, pages: &[PageInfo], cx: &mut Context<Self>) -> impl IntoElement {
        let mut page_grid = div().flex().flex_wrap().gap_2();

        for page in pages {
            let page_number = page.page_number;
            let is_selected = self.selected_page == Some(page_number);
            
            page_grid = page_grid.child(
                div()
                    .size(px(80.0))
                    .bg(page.page_type.color())
                    .when(is_selected, |this| this.border_2().border_color(gpui::rgb(0xffffff)))
                    .when(!is_selected, |this| this.border_1().border_color(gpui::rgb(0x555555)))
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|this| this.opacity(0.8))
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _event, _window, cx| {
                        this.select_page(page_number, cx);
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(gpui::rgb(0xffffff))
                            .child(format!("{}", page.page_number))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(gpui::rgb(0xffffff))
                            .opacity(0.8)
                            .child(page.page_type.short_name())
                    )
            );
        }

        div()
            .flex()
            .flex_1()
            .flex_col()
            .p_4()
            .child(page_grid)
    }

    pub fn register_actions(_cx: &mut Context<Self>) {
        // Action registration would be done at the application level in GPUI
        // For now, we'll handle actions through direct method calls
    }

    fn handle_open_file(&mut self, _action: &OpenFile, cx: &mut Context<Self>) {
        // In a real implementation, this would open a file dialog
        // For now, we'll just refresh the current file if one is open
        if self.current_file_path().is_some() {
            self.refresh_database(cx).detach();
        }
    }

    fn handle_select_page(&mut self, _action: &SelectPage, cx: &mut Context<Self>) {
        // This would be called with a specific page number
        // For now, just select the first page if available
        if let Some(ref db_info) = self.database_info {
            if let Some(first_page) = db_info.pages.first() {
                self.select_page(first_page.page_number, cx);
            }
        }
    }

    fn handle_refresh_database(&mut self, _action: &RefreshDatabase, cx: &mut Context<Self>) {
        self.refresh_database(cx).detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_browser_creation(cx: &mut TestAppContext) {
        let browser = cx.new(|cx| SqliteBrowser::new(cx));
        
        browser.read_with(cx, |browser, _cx| {
            assert!(browser.database_info().is_none());
            assert!(browser.current_file_path().is_none());
            assert!(!browser.is_file_being_watched());
        });
    }

    #[gpui::test]
    async fn test_page_selection(cx: &mut TestAppContext) {
        let browser = cx.new(|cx| SqliteBrowser::new(cx));
        
        browser.update(cx, |browser, cx| {
            // Without database info, selecting a page should do nothing
            browser.select_page(1, cx);
            assert_eq!(browser.selected_page, None);
        });
    }

    #[gpui::test]
    async fn test_status_messages(cx: &mut TestAppContext) {
        let browser = cx.new(|cx| SqliteBrowser::new(cx));
        
        browser.update(cx, |browser, cx| {
            browser.set_status_message("Test message".to_string(), false, cx);
            assert!(browser.status_message.is_some());
            
            browser.clear_status_message(cx);
            assert!(browser.status_message.is_none());
        });
    }
}