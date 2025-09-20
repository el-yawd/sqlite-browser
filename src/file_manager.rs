use crate::models::DatabaseInfo;
use crate::parser::{DatabaseParser, create_sqlite_parser};
use anyhow::Result;
use gpui::{Context, EventEmitter, Task, Timer};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Debug, Clone)]
pub enum FileManagerEvent {
    FileOpened(PathBuf, Arc<DatabaseInfo>),
    FileModified(PathBuf, Arc<DatabaseInfo>),
    FileDeleted(PathBuf),
    ParseError(PathBuf, String),
    WatchingStarted(PathBuf),
    WatchingStopped(PathBuf),
    WatchingFailed(PathBuf, String),
    ParseProgress(PathBuf, f32),
    ParseStarted(PathBuf),
    ParseCompleted(PathBuf),
    ParseCancelled(PathBuf),
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub debounce_duration: Duration,
    pub reload_timeout: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            retry_attempts: 3,
            retry_delay: Duration::from_millis(500),
            debounce_duration: Duration::from_millis(100),
            reload_timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseConfig {
    pub batch_size: usize,
    pub max_parse_time: Duration,
    pub enable_cancellation: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            max_parse_time: Duration::from_secs(5),
            enable_cancellation: true,
        }
    }
}

pub struct FileManager {
    current_file: Option<PathBuf>,
    _watcher: Option<RecommendedWatcher>,
    watcher_config: WatcherConfig,
    parse_config: ParseConfig,
    last_modification: Option<Instant>,
    watching_failed: bool,
    current_parse_cancel: Option<Arc<AtomicBool>>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            current_file: None,
            _watcher: None,
            watcher_config: WatcherConfig::default(),
            parse_config: ParseConfig::default(),
            last_modification: None,
            watching_failed: false,
            current_parse_cancel: None,
        }
    }

    pub fn new_with_config(watcher_config: WatcherConfig, parse_config: ParseConfig) -> Self {
        Self {
            current_file: None,
            _watcher: None,
            watcher_config,
            parse_config,
            last_modification: None,
            watching_failed: false,
            current_parse_cancel: None,
        }
    }

    pub fn set_watcher_config(&mut self, config: WatcherConfig) {
        self.watcher_config = config;
    }

    pub fn set_parse_config(&mut self, config: ParseConfig) {
        self.parse_config = config;
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.current_file.as_deref()
    }

    pub fn is_watching(&self) -> bool {
        self._watcher.is_some() && !self.watching_failed
    }

    pub fn has_watching_failed(&self) -> bool {
        self.watching_failed
    }

    pub fn get_last_modification(&self) -> Option<Instant> {
        self.last_modification
    }

    pub fn open_file<T>(
        &mut self,
        path: PathBuf,
        cx: &mut Context<T>,
    ) -> Task<Result<Arc<DatabaseInfo>>>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        self.open_file_with_progress(path, cx)
    }

    pub fn open_file_with_progress<T>(
        &mut self,
        path: PathBuf,
        cx: &mut Context<T>,
    ) -> Task<Result<Arc<DatabaseInfo>>>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        let path_clone = path.clone();
        
        // Create cancellation flag
        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.current_parse_cancel = Some(cancel_flag.clone());

        // Emit parse started event
        cx.emit(FileManagerEvent::ParseStarted(path.clone()));

        cx.spawn(async move |entity, cx| {
            let parser = create_sqlite_parser();
            
            // Simple parsing with progress indication
            let result = parser.parse_file(&path_clone);
            
            // Emit completion events
            let _ = entity.update(cx, |_this, cx| {
                match &result {
                    Ok(_) => {
                        cx.emit(FileManagerEvent::ParseCompleted(path_clone.clone()));
                    }
                    Err(e) => {
                        cx.emit(FileManagerEvent::ParseError(path_clone.clone(), e.to_string()));
                    }
                }
            });
            
            result
        })
    }

    pub fn cancel_current_parse(&mut self) {
        if let Some(ref cancel_flag) = self.current_parse_cancel {
            cancel_flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn is_parsing(&self) -> bool {
        self.current_parse_cancel.is_some()
    }

    pub fn set_current_file(&mut self, path: Option<PathBuf>) {
        self.current_file = path;
    }

    pub fn start_watching<T>(&mut self, path: &Path, cx: &mut Context<T>) -> Result<()>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        // Reset failure state
        self.watching_failed = false;

        // Try to start watching with retry logic
        self.try_start_watching(path, cx, 0)
    }

    fn try_start_watching<T>(&mut self, path: &Path, cx: &mut Context<T>, attempt: u32) -> Result<()>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        let (sync_tx, sync_rx) = mpsc::channel();
        let (async_tx, mut async_rx) = tokio_mpsc::unbounded_channel();

        // Try to create and configure watcher
        let watcher_result = recommended_watcher(sync_tx)
            .and_then(|mut w| {
                w.watch(path, RecursiveMode::NonRecursive)?;
                Ok(w)
            });

        let watcher = match watcher_result {
            Ok(w) => w,
            Err(e) => {
                if attempt < self.watcher_config.retry_attempts {
                    eprintln!("Failed to start file watcher (attempt {}): {}", attempt + 1, e);
                    
                    // Schedule retry
                    let path_clone = path.to_path_buf();
                    let retry_delay = self.watcher_config.retry_delay;
                    let next_attempt = attempt + 1;
                    
                    cx.spawn(async move |entity, cx| {
                        Timer::after(retry_delay).await;
                        // We can't call try_start_watching from here because we don't have access to self
                        // Instead, emit a retry event that the parent can handle
                        let _ = entity.update(cx, |_this, cx| {
                            cx.emit(FileManagerEvent::WatchingFailed(
                                path_clone,
                                format!("Retrying file watching (attempt {})...", next_attempt + 1)
                            ));
                        });
                    }).detach();
                    
                    return Ok(());
                } else {
                    self.watching_failed = true;
                    cx.emit(FileManagerEvent::WatchingFailed(
                        path.to_path_buf(),
                        format!("Failed to start watching after {} attempts: {}", self.watcher_config.retry_attempts, e)
                    ));
                    return Err(e.into());
                }
            }
        };

        self._watcher = Some(watcher);
        self.watching_failed = false;

        // Emit watching started event
        eprintln!("DEBUG: File watching started for: {}", path.display());
        cx.emit(FileManagerEvent::WatchingStarted(path.to_path_buf()));

        // Bridge sync channel to async channel in background thread
        std::thread::spawn(move || {
            while let Ok(event) = sync_rx.recv() {
                if async_tx.send(event).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        // Spawn task to handle file change events with debouncing and enhanced error handling
        let path_clone = path.to_path_buf();
        let debounce_duration = self.watcher_config.debounce_duration;
        let _reload_timeout = self.watcher_config.reload_timeout;
        
        cx.spawn(async move |entity, cx| {
            let parser = create_sqlite_parser();
            let mut last_event_time: Option<Instant> = None;
            let mut consecutive_errors = 0u32;
            const MAX_CONSECUTIVE_ERRORS: u32 = 5;

            loop {
                match async_rx.recv().await {
                    Some(event_result) => {
                        eprintln!("DEBUG: Received file event: {:?}", event_result);
                        match event_result {
                            Ok(Event {
                                kind: EventKind::Modify(_),
                                ..
                            })
                            | Ok(Event {
                                kind: EventKind::Create(_),
                                ..
                            }) => {
                                let now = Instant::now();
                                
                                // Debounce rapid file changes
                                if let Some(last_time) = last_event_time {
                                    if now.duration_since(last_time) < debounce_duration {
                                        continue; // Skip this event, too soon after last one
                                    }
                                }
                                last_event_time = Some(now);

                                // Wait for debounce period to ensure file is fully written
                                Timer::after(debounce_duration).await;

                                // Parse file - use simple parsing for file watching
                                let parse_result = parser.parse_file(&path_clone);

                                match parse_result {
                                    Ok(database_info) => {
                                        consecutive_errors = 0; // Reset error count on success
                                        
                                        // File was modified - emit event to update UI
                                        if entity.update(cx, |_this, cx| {
                                            eprintln!("File modified and reloaded: {}", path_clone.display());
                                            cx.emit(FileManagerEvent::FileModified(
                                                path_clone.clone(),
                                                database_info,
                                            ));
                                        }).is_err() {
                                            eprintln!("DEBUG: Failed to emit FileModified event - entity dropped");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        consecutive_errors += 1;
                                        eprintln!("Parse error (attempt {}): {}", consecutive_errors, e);
                                        
                                        // Emit parse error event
                                        if entity.update(cx, |_this, cx| {
                                            cx.emit(FileManagerEvent::ParseError(
                                                path_clone.clone(),
                                                format!("Failed to reload file: {}", e),
                                            ));
                                        }).is_err() {
                                            break;
                                        }

                                        // If too many consecutive errors, disable watching
                                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                                            if entity.update(cx, |_this, cx| {
                                                cx.emit(FileManagerEvent::WatchingFailed(
                                                    path_clone.clone(),
                                                    format!("Too many consecutive parse errors ({}), disabling file watching", consecutive_errors)
                                                ));
                                            }).is_err() {
                                                // Entity dropped
                                            }
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
                                if entity.update(cx, |_this, cx| {
                                    cx.emit(FileManagerEvent::FileDeleted(path_clone.clone()));
                                    cx.emit(FileManagerEvent::WatchingStopped(path_clone.clone()));
                                }).is_err() {
                                    // Entity dropped
                                }
                                break;
                            }
                            Ok(_) => {
                                // Ignore other events
                            }
                            Err(e) => {
                                consecutive_errors += 1;
                                eprintln!("File watcher error (attempt {}): {}", consecutive_errors, e);
                                
                                // If too many consecutive watcher errors, disable watching
                                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                                    if entity.update(cx, |_this, cx| {
                                        cx.emit(FileManagerEvent::WatchingFailed(
                                            path_clone.clone(),
                                            format!("Too many consecutive watcher errors ({}): {}", consecutive_errors, e)
                                        ));
                                    }).is_err() {
                                        // Entity dropped
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    None => {
                        eprintln!("File watcher channel closed");
                        if entity.update(cx, |_this, cx| {
                            cx.emit(FileManagerEvent::WatchingStopped(path_clone.clone()));
                        }).is_err() {
                            // Entity dropped
                        }
                        break;
                    }
                }
            }
        })
        .detach();

        Ok(())
    }

    pub fn stop_watching<T>(&mut self, cx: &mut Context<T>)
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        if let Some(path) = &self.current_file {
            cx.emit(FileManagerEvent::WatchingStopped(path.clone()));
        }
        self._watcher = None;
        self.watching_failed = false;
    }

    pub fn stop_watching_silent(&mut self) {
        self._watcher = None;
        self.watching_failed = false;
    }

    pub fn refresh_current_file<T>(&mut self, cx: &mut Context<T>) -> Task<Result<Arc<DatabaseInfo>>>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        if let Some(path) = self.current_file.clone() {
            self.open_file_with_progress(path, cx)
        } else {
            Task::ready(Err(anyhow::anyhow!("No file currently open")))
        }
    }

    // Method to handle retry logic from external events
    pub fn retry_watching<T>(&mut self, path: &Path, cx: &mut Context<T>) -> Result<()>
    where
        T: EventEmitter<FileManagerEvent> + 'static,
    {
        if !self.watching_failed {
            self.start_watching(path, cx)
        } else {
            Err(anyhow::anyhow!("File watching has failed and cannot be retried"))
        }
    }

    // Update last modification time (called from browser when file is successfully reloaded)
    pub fn update_last_modification(&mut self, time: Instant) {
        self.last_modification = Some(time);
    }

    // Mark watching as failed (called from browser when handling WatchingFailed events)
    pub fn mark_watching_failed(&mut self) {
        self.watching_failed = true;
        self._watcher = None;
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
        assert!(!file_manager.has_watching_failed());
    }

    #[test]
    fn test_file_manager_with_config() {
        let watcher_config = WatcherConfig {
            retry_attempts: 5,
            retry_delay: Duration::from_millis(1000),
            debounce_duration: Duration::from_millis(200),
            reload_timeout: Duration::from_secs(3),
        };
        let parse_config = ParseConfig {
            batch_size: 500,
            max_parse_time: Duration::from_secs(10),
            enable_cancellation: false,
        };

        let file_manager = FileManager::new_with_config(watcher_config.clone(), parse_config.clone());

        assert_eq!(file_manager.watcher_config.retry_attempts, 5);
        assert_eq!(file_manager.watcher_config.retry_delay, Duration::from_millis(1000));
        assert_eq!(file_manager.parse_config.batch_size, 500);
        assert!(!file_manager.parse_config.enable_cancellation);
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

    #[test]
    fn test_config_updates() {
        let mut file_manager = FileManager::new();
        
        let new_watcher_config = WatcherConfig {
            retry_attempts: 10,
            retry_delay: Duration::from_millis(2000),
            debounce_duration: Duration::from_millis(300),
            reload_timeout: Duration::from_secs(5),
        };
        
        file_manager.set_watcher_config(new_watcher_config.clone());
        assert_eq!(file_manager.watcher_config.retry_attempts, 10);
        assert_eq!(file_manager.watcher_config.retry_delay, Duration::from_millis(2000));
    }

    #[test]
    fn test_default_configs() {
        let watcher_config = WatcherConfig::default();
        assert_eq!(watcher_config.retry_attempts, 3);
        assert_eq!(watcher_config.retry_delay, Duration::from_millis(500));
        assert_eq!(watcher_config.debounce_duration, Duration::from_millis(100));
        assert_eq!(watcher_config.reload_timeout, Duration::from_secs(2));

        let parse_config = ParseConfig::default();
        assert_eq!(parse_config.batch_size, 1000);
        assert_eq!(parse_config.max_parse_time, Duration::from_secs(5));
        assert!(parse_config.enable_cancellation);
    }

    #[test]
    fn test_watching_state_management() {
        let mut file_manager = FileManager::new();
        
        assert!(!file_manager.is_watching());
        assert!(!file_manager.has_watching_failed());
        assert!(!file_manager.is_parsing());
        
        file_manager.mark_watching_failed();
        assert!(file_manager.has_watching_failed());
        assert!(!file_manager.is_watching());
        
        file_manager.stop_watching_silent();
        assert!(!file_manager.has_watching_failed());
        assert!(!file_manager.is_watching());
    }
}