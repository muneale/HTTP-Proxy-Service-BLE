use bluer::UuidExt;
use event_emitter_rs::EventEmitter;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

pub const MTU_OVERHEAD: usize = 3;
pub const HTTP_STATUS_CODE_UPDATED_EVENT: &str = "http_status_code_updated";

pub static EVENT_EMITTER: Lazy<Mutex<EventEmitter>> = Lazy::new(|| Mutex::new(EventEmitter::new()));

pub static SERVICE_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x1823));
pub static HTTP_URI_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB6));
pub static HTTP_HEADERS_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB7));
pub static HTTP_STATUS_CODE_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB8));
pub static HTTP_ENTITY_BODY_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB9));
pub static HTTP_CONTROL_POINT_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2ABA));
pub static HTTPS_SECURITY_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2ABB));
pub static HTTP_HEADERS_BODY_CHUNK_IDX_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2A9A));
pub static HTTP_HEADERS_BODY_SIZES_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AC0));
