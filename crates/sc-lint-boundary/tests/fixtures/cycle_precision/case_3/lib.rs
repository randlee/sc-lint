pub trait Connect {
    fn connect(&self) -> u32;
}

pub struct GraftClient {
    pub port: u32,
}

impl GraftClient {
    pub fn connect(&self) -> u32 {
        self.port
    }
}

impl Connect for GraftClient {
    fn connect(&self) -> u32 {
        self.connect()
    }
}
