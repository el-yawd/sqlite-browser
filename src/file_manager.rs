use anyhow::Result;
use gpui::{Context, Task};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use crate::models::DatabaseInfo;
use crate::parser::{create_sqlite_parser, DatabaseParser};

#[derive(Debug, Clone)]
pub enum FileManagerEvent {
    FileOpened(PathBuf, DatabaseInfo),
    FileModified(PathBuf, DatabaseInfo),
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
        cx: &mut Context<T>
    ) -> Task<Result<DatabaseInfo>>
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

    pub fn start_watching<T>(
        &mut self, 
        path: &Path, 
        cx: &mut Context<T>
    ) -> Result<()>
    where
        T: 'static,
    {
        let (tx, rx) = mpsc::channel();

        let mut watcher = recommended_watcher(tx)?;
        watcher.watch(path, RecursiveMode::NonRecursive)?;

        self._watcher = Some(watcher);

        // Spawn task to handle file change events
        let path_clone = path.to_path_buf();
        cx.spawn(async move |entity, cx| {
            let parser = create_sqlite_parser();
            
            loop {
                match rx.recv() {
                    Ok(event_result) => {
                        match event_result {
                            Ok(Event { kind: EventKind::Modify(_), .. }) |
                            Ok(Event { kind: EventKind::Create(_), .. }) => {
                                // File was modified, re-parse it
                                match parser.parse_file(&path_clone).await {
                                    Ok(database_info) => {
                                        // File was modified - the browser will handle this through other means
                                        eprintln!("File {} was modified", path_clone.display());
                                    }
                                    Err(e) => {
                                        eprintln!("Error re-parsing file {}: {}", path_clone.display(), e);
                                    }
                                }
                            }
                            Ok(Event { kind: EventKind::Remove(_), .. }) => {
                                // File was deleted
                                eprintln!("File {} was deleted", path_clone.display());
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
                    Err(e) => {
                        eprintln!("File watcher channel error: {}", e);
                        break;
                    }
                }
            }
        }).detach();

        Ok(())
    }

    pub fn stop_watching(&mut self) {
        self._watcher = None;
    }

    pub fn refresh_current_file<T>(
        &self, 
        cx: &mut Context<T>
    ) -> Task<Result<DatabaseInfo>>
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
    use tempfile::NamedTempFile;

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