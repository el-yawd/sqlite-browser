use crate::file_manager::{FileManager, FileManagerEvent};
use crate::models::DatabaseInfo;
use anyhow::Result;
use gpui::{
    Context, EventEmitter, IntoElement, ParentElement, Render, Task, Window, div, prelude::*, rgb,
};
use rfd::FileDialog;
use std::{path::PathBuf, sync::Arc};

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
pub enum FileDialogState {
    Idle,
    ShowingDialog,
    Loading(PathBuf),
    Error(String),
}

pub struct FileDialogManager {
    file_manager: FileManager,
    state: FileDialogState,
}

impl EventEmitter<FileOpenRequested> for FileDialogManager {}
impl EventEmitter<FileOpened> for FileDialogManager {}
impl EventEmitter<FileOpenError> for FileDialogManager {}
impl EventEmitter<FileManagerEvent> for FileDialogManager {}

impl FileDialogManager {
    pub fn new() -> Self {
        Self {
            file_manager: FileManager::new(),
            state: FileDialogState::Idle,
        }
    }

    pub fn open_file_dialog(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.state = FileDialogState::ShowingDialog;
        cx.notify();

        cx.spawn(async move |entity, cx| {
            let file_dialog = FileDialog::new()
                .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
                .set_title("Open SQLite Database");

            if let Some(file_path) = file_dialog.pick_file() {
                entity.update(cx, |this, cx| {
                    this.open_file(file_path, cx).detach();
                })?;
            } else {
                entity.update(cx, |this, cx| {
                    this.state = FileDialogState::Idle;
                    cx.notify();
                })?;
            }
            Ok(())
        })
    }

    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) -> Task<Result<()>> {
        self.state = FileDialogState::Loading(path.clone());
        cx.notify();

        // Emit file open requested event
        // cx.emit(FileOpenRequested { path: path.clone() });

        let parse_task = self.file_manager.open_file(path.clone(), cx);

        cx.spawn(async move |entity, cx| match parse_task.await {
            Ok(database_info) => {
                entity.update(cx, |this, cx| {
                    this.file_manager.set_current_file(Some(path.clone()));
                    this.state = FileDialogState::Idle;
                    cx.emit(FileOpened {
                        path: path.clone(),
                        database_info: database_info.clone(),
                    });
                    cx.notify();
                })?;
                Ok(())
            }
            Err(e) => {
                entity.update(cx, |this, cx| {
                    this.state = FileDialogState::Error(e.to_string());
                    cx.emit(FileOpenError {
                        path: path.clone(),
                        error: e.to_string(),
                    });
                    cx.notify();
                })?;
                Err(e)
            }
        })
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
