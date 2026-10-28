use std::path::PathBuf;

pub struct SqliteStorageFactory {
    pub root: PathBuf,
    pub retries: u32,
}

impl Clone for SqliteStorageFactory {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            retries: self.retries,
        }
    }
}
