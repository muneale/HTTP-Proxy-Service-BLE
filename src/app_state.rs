use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedBuffer = Arc<Mutex<Vec<u8>>>;

pub struct AppState {
    pub http_uri: SharedBuffer,
    pub http_headers: SharedBuffer,
    pub http_status_code: SharedBuffer,
    pub http_entity_body: SharedBuffer,
    pub https_security: SharedBuffer,
    pub http_headers_body_chunk_idx: SharedBuffer,
    pub http_headers_body_sizes: SharedBuffer,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            http_uri: Arc::new(Mutex::new(Vec::new())),
            http_headers: Arc::new(Mutex::new(Vec::new())),
            http_status_code: Arc::new(Mutex::new(Vec::new())),
            http_entity_body: Arc::new(Mutex::new(Vec::new())),
            https_security: Arc::new(Mutex::new(Vec::new())),
            http_headers_body_chunk_idx: Arc::new(Mutex::new(vec![0; 8])),
            http_headers_body_sizes: Arc::new(Mutex::new(Vec::new())),
        }
    }
}