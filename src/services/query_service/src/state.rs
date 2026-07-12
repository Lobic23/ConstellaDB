use std::sync::Arc;
use tokio::sync::Mutex;
use db_module::Engine;

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