pub trait Runtime {
    fn start(&self) -> bool;
}

pub struct LocalServiceRuntime {
    pub started: bool,
}

impl LocalServiceRuntime {
    pub fn start(&self) -> bool {
        self.started
    }
}

impl Runtime for LocalServiceRuntime {
    fn start(&self) -> bool {
        self.start()
    }
}
