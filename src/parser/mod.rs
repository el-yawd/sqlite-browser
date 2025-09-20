pub mod sqlite_parser;

use crate::models::DatabaseInfo;
use anyhow::Result;
use std::{path::Path, sync::Arc};
use std::sync::atomic::AtomicBool;

pub use sqlite_parser::{ProgressCallback, BatchParseConfig};

pub trait DatabaseParser {
    fn parse_file<P: AsRef<Path> + Send>(&self, path: P) -> Result<Arc<DatabaseInfo>>;
    
    fn parse_file_with_progress<P: AsRef<Path> + Send>(
        &self,
        path: P,
        progress_callback: Option<ProgressCallback>,
        cancel_flag: Option<Arc<AtomicBool>>,
        config: Option<BatchParseConfig>,
    ) -> Result<Arc<DatabaseInfo>>;
}

pub struct SqliteParser;

impl DatabaseParser for SqliteParser {
    fn parse_file<P: AsRef<Path> + Send>(&self, path: P) -> Result<Arc<DatabaseInfo>> {
        sqlite_parser::parse_database_file(path.as_ref())
    }
    
    fn parse_file_with_progress<P: AsRef<Path> + Send>(
        &self,
        path: P,
        progress_callback: Option<ProgressCallback>,
        cancel_flag: Option<Arc<AtomicBool>>,
        config: Option<BatchParseConfig>,
    ) -> Result<Arc<DatabaseInfo>> {
        sqlite_parser::parse_database_file_with_progress(
            path.as_ref(),
            progress_callback,
            cancel_flag,
            config,
        )
    }
}

pub fn create_sqlite_parser() -> SqliteParser {
    SqliteParser
}
