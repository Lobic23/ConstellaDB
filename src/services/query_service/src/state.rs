use db_module::Engine;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub engine: Arc<Mutex<Engine>>,
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
        }
    }
}
