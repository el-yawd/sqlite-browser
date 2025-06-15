pub mod sqlite_parser;

use crate::models::DatabaseInfo;
use anyhow::Result;
use std::{path::Path, sync::Arc};

pub trait DatabaseParser {
    fn parse_file<P: AsRef<Path> + Send>(
        &self,
        path: P,
    ) -> impl std::future::Future<Output = Result<Arc<DatabaseInfo>>> + Send;
}

pub struct SqliteParser;

impl DatabaseParser for SqliteParser {
    async fn parse_file<P: AsRef<Path> + Send>(&self, path: P) -> Result<Arc<DatabaseInfo>> {
        sqlite_parser::parse_database_file(path.as_ref()).await
    }
}

pub fn create_sqlite_parser() -> SqliteParser {
    SqliteParser
}
