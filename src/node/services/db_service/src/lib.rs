use std::sync::{Arc, Mutex};

use constella_db::modules::db::Engine;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Mutex<Engine>>,
}