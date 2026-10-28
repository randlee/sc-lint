pub trait Route {
    fn route(&self, key: &str) -> usize;
}

pub struct StorageAndNudgeRouter {
    pub shards: usize,
}

impl StorageAndNudgeRouter {
    pub fn route(&self, key: &str) -> usize {
        key.len() % self.shards.max(1)
    }
}

impl Route for StorageAndNudgeRouter {
    fn route(&self, key: &str) -> usize {
        self.route(key)
    }
}
