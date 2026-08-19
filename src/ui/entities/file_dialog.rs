use crate::file_manager::{FileManager, FileManagerEvent, ParseOperationError};
use crate::models::DatabaseInfo;
use anyhow::Result;
use gpui::{
    Context, EventEmitter, IntoElement, ParentElement, Render, Task, Window, div, prelude::*, rgb,
};
use rfd::FileDialog;
use std::{path::PathBuf, sync::Arc};

pub trait FilePicker: Send + Sync {
    fn pick_file(&self) -> Result<Option<PathBuf>>;
}

struct NativeFilePicker;

impl FilePicker for NativeFilePicker {
    fn pick_file(&self) -> Result<Option<PathBuf>> {
        Ok(FileDialog::new()
            .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
            .set_title("Open SQLite Database")
            .pick_file())
    }
}

#[derive(Clone, Debug)]
pub struct FileOpenRequested {
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FileOpened {
    pub path: PathBuf,
    pub database_info: Arc<DatabaseInfo>,
}

#[derive(Clone, Debug)]
pub struct FileOpenError {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Clone, Debug)]
pub struct FilePickerError {
    pub error: String,
}

#[derive(Clone, Debug)]
pub enum FileDialogState {
    Idle,
    ShowingDialog,
    Loading(PathBuf),
    Error(String),
}

pub struct FileDialogManager {
    file_manager: FileManager,
    state: FileDialogState,
    load_generation: u64,
    file_picker: Arc<dyn FilePicker>,
}

impl EventEmitter<FileOpenRequested> for FileDialogManager {}
impl EventEmitter<FileOpened> for FileDialogManager {}
impl EventEmitter<FileOpenError> for FileDialogManager {}
impl EventEmitter<FilePickerError> for FileDialogManager {}
impl EventEmitter<FileManagerEvent> for FileDialogManager {}

impl FileDialogManager {
    pub fn new() -> Self {
        Self {
            file_manager: FileManager::new(),
            state: FileDialogState::Idle,
            load_generation: 0,
            file_picker: Arc::new(NativeFilePicker),
        }
    }

    pub fn with_file_picker(file_picker: Arc<dyn FilePicker>) -> Self {
        Self {
            file_manager: FileManager::new(),
            state: FileDialogState::Idle,
            load_generation: 0,
            file_picker,
        }
    }

    pub fn open_file_dialog(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.state = FileDialogState::ShowingDialog;
        cx.notify();

        let file_picker = self.file_picker.clone();
        let pick_task = cx
            .background_executor()
            .spawn(async move { file_picker.pick_file() });
        cx.spawn(async move |entity, cx| {
            match pick_task.await {
                Ok(Some(file_path)) => {
                    entity.update(cx, |this, cx| {
                        this.open_file(file_path, cx).detach();
                    })?;
                }
                Ok(None) => {
                    entity.update(cx, |this, cx| {
                        this.state = FileDialogState::Idle;
                        cx.notify();
                    })?;
                }
                Err(error) => {
                    entity.update(cx, |this, cx| {
                        this.state = FileDialogState::Error(error.to_string());
                        cx.emit(FilePickerError {
                            error: error.to_string(),
                        });
                        cx.notify();
                    })?;
                    return Err(error);
                }
            }
            Ok(())
        })
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.load_generation = self.load_generation.wrapping_add(1);
        let load_generation = self.load_generation;
        self.state = FileDialogState::Loading(path.clone());
        cx.notify();

        // Emit file open requested event
        // cx.emit(FileOpenRequested { path: path.clone() });

        let parse_task = self.file_manager.open_file(path.clone(), cx);

        cx.spawn(async move |entity, cx| {
            let result = parse_task.await;
            entity.update(cx, |this, cx| {
                if this.load_generation != load_generation {
                    return Ok(());
                }

                match result {
                    Ok(database_info) => {
                        this.file_manager.set_current_file(Some(path.clone()));
                        this.state = FileDialogState::Idle;
                        cx.emit(FileOpened {
                            path: path.clone(),
                            database_info,
                        });
                        cx.notify();
                        Ok(())
                    }
                    Err(error)
                        if error
                            .downcast_ref::<ParseOperationError>()
                            .is_some_and(|error| {
                                matches!(
                                    error,
                                    ParseOperationError::Cancelled
                                        | ParseOperationError::Superseded
                                )
                            }) =>
                    {
                        this.state = FileDialogState::Idle;
                        cx.notify();
                        Ok(())
                    }
                    Err(error) => {
                        this.state = FileDialogState::Error(error.to_string());
                        cx.emit(FileOpenError {
                            path: path.clone(),
                            error: error.to_string(),
                        });
                        cx.notify();
                        Err(error)
                    }
                }
            })?
        })
    }

    pub fn cancel_current_parse(&mut self) {
        self.file_manager.cancel_current_parse();
    }

    pub fn is_parsing(&self) -> bool {
        self.file_manager.is_parsing()
    }

    pub fn try_open_file_or_dialog(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        if path.exists() && path.is_file() {
            self.open_file(path, cx)
        } else {
            self.open_file_dialog(cx)
        }
    }

    pub fn state(&self) -> &FileDialogState {
        &self.state
    }

    pub fn current_file(&self) -> Option<&std::path::Path> {
        self.file_manager.current_file()
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state, FileDialogState::Loading(_))
    }

    pub fn clear_error(&mut self, cx: &mut Context<Self>) {
        if matches!(self.state, FileDialogState::Error(_)) {
            self.state = FileDialogState::Idle;
            cx.notify();
        }
    }
}

impl Render for FileDialogManager {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.state {
            FileDialogState::Idle => div()
                .id("file-dialog-idle")
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .p_8()
                .child(
                    div()
                        .text_xl()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child("SQLite Browser"),
                )
                .child(
                    div()
                        .text_color(rgb(0xaaaaaa))
                        .child("Open a SQLite database file to get started"),
                )
                .child(
                    div()
                        .debug_selector(|| "file-dialog-open".to_string())
                        .px_6()
                        .py_3()
                        .bg(rgb(0x007acc))
                        .hover(|this| this.bg(rgb(0x005a9e)))
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
                                .text_color(rgb(0xffffff))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child("Open Database"),
                        ),
                ),
            FileDialogState::ShowingDialog => div()
                .id("file-dialog-showing")
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_color(rgb(0xaaaaaa))
                        .child("Opening file dialog..."),
                ),
            FileDialogState::Loading(path) => div()
                .id("file-dialog-loading")
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .child(div().text_color(rgb(0xffffff)).child("Loading database..."))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0xaaaaaa))
                        .child(format!("{}", path.display())),
                ),
            FileDialogState::Error(error) => div()
                .id("file-dialog-error")
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .p_8()
                .child(
                    div()
                        .text_lg()
                        .text_color(rgb(0xff6b6b))
                        .child("Error loading database"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0xaaaaaa))
                        .child(error.clone()),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .child(
                            div()
                                .debug_selector(|| "file-dialog-retry".to_string())
                                .px_4()
                                .py_2()
                                .bg(rgb(0x007acc))
                                .hover(|this| this.bg(rgb(0x005a9e)))
                                .rounded_md()
                                .cursor_pointer()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|this, _event, _window, cx| {
                                        this.open_file_dialog(cx).detach();
                                    }),
                                )
                                .child(div().text_color(rgb(0xffffff)).child("Try Again")),
                        )
                        .child(
                            div()
                                .debug_selector(|| "file-dialog-dismiss".to_string())
                                .px_4()
                                .py_2()
                                .border_1()
                                .border_color(rgb(0x555555))
                                .hover(|this| this.bg(rgb(0x3e3e3e)))
                                .rounded_md()
                                .cursor_pointer()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|this, _event, _window, cx| {
                                        this.clear_error(cx);
                                    }),
                                )
                                .child(div().text_color(rgb(0xffffff)).child("Dismiss")),
                        ),
                ),
        }
    }
}

impl Default for FileDialogManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext};

    struct CancelledFilePicker;

    impl FilePicker for CancelledFilePicker {
        fn pick_file(&self) -> Result<Option<PathBuf>> {
            Ok(None)
        }
    }

    #[gpui::test]
    fn dismissing_an_error_returns_to_the_open_state(cx: &mut TestAppContext) {
        let (dialog, cx) = cx.add_window_view(|_window, _cx| FileDialogManager::new());
        dialog.update(cx, |dialog, cx| {
            dialog.state = FileDialogState::Error("Invalid SQLite header".to_string());
            cx.notify();
        });
        cx.update(|window, _cx| window.refresh());

        let dismiss_bounds = cx
            .debug_bounds("file-dialog-dismiss")
            .expect("dismiss button should be rendered");
        cx.simulate_click(dismiss_bounds.center(), Modifiers::default());

        assert!(cx.read(|cx| matches!(dialog.read(cx).state(), FileDialogState::Idle)));
        assert!(cx.debug_bounds("file-dialog-open").is_some());
    }

    #[gpui::test]
    async fn cancelling_the_native_picker_returns_to_idle(cx: &mut TestAppContext) {
        let dialog =
            cx.new(|_cx| FileDialogManager::with_file_picker(Arc::new(CancelledFilePicker)));

        let task = dialog.update(cx, |dialog, cx| dialog.open_file_dialog(cx));
        task.await
            .expect("cancelling file selection should succeed");

        assert!(cx.read(|cx| matches!(dialog.read(cx).state(), FileDialogState::Idle)));
    }
}
