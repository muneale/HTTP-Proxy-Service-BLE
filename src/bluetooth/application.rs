use crate::{AppState, Config};
use bluer::gatt::local::{Application, Service};
use std::sync::Arc;
use crate::constants::SERVICE_UUID;
use super::characteristics;

pub fn create_application(state: &Arc<AppState>, config: &Config) -> Application {
    Application {
        services: vec![Service {
            uuid: *SERVICE_UUID,
            primary: true,
            characteristics: vec![
                characteristics::create_headers_body_mtu_sizes(state),
                characteristics::create_headers_body_chunk_idx(state),
                characteristics::create_http_uri(state),
                characteristics::create_http_headers(state, config),
                characteristics::create_http_status_code(state),
                characteristics::create_http_entity_body(state, config),
                characteristics::create_https_security(state),
                characteristics::create_http_control_point(state, config),
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}