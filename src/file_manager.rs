use crate::models::DatabaseInfo;
use crate::parser::{DatabaseParser, create_sqlite_parser};
use anyhow::Result;
use gpui::{Context, EventEmitter, Task};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Debug, Clone)]
pub enum FileManagerEvent {
    FileOpened(PathBuf, Arc<DatabaseInfo>),
    FileModified(PathBuf, Arc<DatabaseInfo>),
    FileDeleted(PathBuf),
    ParseError(PathBuf, String),
}

pub struct FileManager {
    current_file: Option<PathBuf>,
    _watcher: Option<RecommendedWatcher>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            current_file: None,
            _watcher: None,
        }
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.current_file.as_deref()
    }

    pub fn is_watching(&self) -> bool {
        self._watcher.is_some()
    }

    pub fn open_file<T>(
        &mut self,
        path: PathBuf,
        cx: &mut Context<T>,
    ) -> Task<Result<Arc<DatabaseInfo>>>
    where
        T: 'static,
    {
        let path_clone = path.clone();

        cx.spawn(async move |_entity, _cx| {
            let parser = create_sqlite_parser();
            match parser.parse_file(&path_clone).await {
                Ok(database_info) => Ok(database_info),
                Err(e) => Err(e),
            }
        })
    }

    pub fn set_current_file(&mut self, path: Option<PathBuf>) {
        self.current_file = path;
    }

    pub fn start_watching<T>(&mut self, path: &Path, cx: &mut Context<T>) -> Result<()>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        let (sync_tx, sync_rx) = mpsc::channel();
        let (async_tx, mut async_rx) = tokio_mpsc::unbounded_channel();

        let mut watcher = recommended_watcher(sync_tx)?;
        watcher.watch(path, RecursiveMode::NonRecursive)?;

        self._watcher = Some(watcher);

        // Bridge sync channel to async channel in background thread
        std::thread::spawn(move || {
            while let Ok(event) = sync_rx.recv() {
                if async_tx.send(event).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        // Spawn task to handle file change events
        let path_clone = path.to_path_buf();
        cx.spawn(async move |entity, cx| {
            let parser = create_sqlite_parser();

            loop {
                match async_rx.recv().await {
                    Some(event_result) => {
                        match event_result {
                            Ok(Event {
                                kind: EventKind::Modify(_),
                                ..
                            })
                            | Ok(Event {
                                kind: EventKind::Create(_),
                                ..
                            }) => {
                                // File was modified, re-parse it
                                match parser.parse_file(&path_clone).await {
                                    Ok(database_info) => {
                                        // File was modified - emit event to update UI
                                        if let Ok(()) = entity.update(cx, |_this, cx| {
                                            eprintln!("File modified: {}", path_clone.display());
                                            cx.emit(FileManagerEvent::FileModified(
                                                path_clone.clone(),
                                                database_info,
                                            ));
                                        }) {
                                            // Successfully updated entity
                                        } else {
                                            eprintln!("DEBUG: Failed to emit ParseError event - entity dropped");
                                            // Entity was dropped, stop watching
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        // Emit parse error event
                                        if let Ok(()) = entity.update(cx, |_this, cx| {
                                            cx.emit(FileManagerEvent::ParseError(
                                                path_clone.clone(),
                                                e.to_string(),
                                            ));
                                        }) {
                                            // Successfully updated entity
                                        } else {
                                            // Entity was dropped, stop watching
                                            break;
                                        }
                                    }
                                }
                            }
                            Ok(Event {
                                kind: EventKind::Remove(_),
                                ..
                            }) => {
                                // File was deleted - emit event and stop watching
                                if let Ok(()) = entity.update(cx, |_this, cx| {
                                    cx.emit(FileManagerEvent::FileDeleted(path_clone.clone()));
                                }) {
                                    // Successfully updated entity
                                }
                                break;
                            }
                            Ok(_) => {
                                // Ignore other events
                            }
                            Err(e) => {
                                eprintln!("File watcher error: {}", e);
                                break;
                            }
                        }
                    }
                    None => {
                        eprintln!("File watcher channel closed");
                        break;
                    }
                }
            }
        })
        .detach();

        Ok(())
    }

    pub fn stop_watching(&mut self) {
        self._watcher = None;
    }

    pub fn refresh_current_file<T>(&self, cx: &mut Context<T>) -> Task<Result<Arc<DatabaseInfo>>>
    where
        T: 'static,
    {
        if let Some(path) = self.current_file.clone() {
            cx.spawn(async move |_entity, _cx| {
                let parser = create_sqlite_parser();
                parser.parse_file(&path).await
            })
        } else {
            Task::ready(Err(anyhow::anyhow!("No file currently open")))
        }
    }
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_manager_creation() {
        let file_manager = FileManager::new();

        assert!(file_manager.current_file().is_none());
        assert!(!file_manager.is_watching());
    }

    #[test]
    fn test_current_file_management() {
        let mut file_manager = FileManager::new();
        let test_path = PathBuf::from("/test/path.db");

        file_manager.set_current_file(Some(test_path.clone()));
        assert_eq!(file_manager.current_file(), Some(test_path.as_path()));

        file_manager.set_current_file(None);
        assert!(file_manager.current_file().is_none());
    }
}
