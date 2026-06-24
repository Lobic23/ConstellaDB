use std::sync::{Arc, Mutex};

use db_module::Engine;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Mutex<Engine>>,
}