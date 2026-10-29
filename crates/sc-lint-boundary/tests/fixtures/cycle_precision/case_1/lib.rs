use std::sync::Arc;

pub struct DaemonObservability {
    pub name: String,
    pub tags: Vec<String>,
    pub shared: Arc<String>,
}

impl Clone for DaemonObservability {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            tags: self.tags.clone(),
            shared: self.shared.clone(),
        }
    }
}
