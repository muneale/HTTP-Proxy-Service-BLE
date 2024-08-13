use crate::AppState;
use bluer::gatt::local::{Characteristic, CharacteristicRead};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;
use crate::constants::HTTP_HEADERS_BODY_SIZES_UUID;

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state = state.clone();
    Characteristic {
        uuid: *HTTP_HEADERS_BODY_SIZES_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state.http_headers_body_sizes.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target: "headers_body_mtu_sizes", "Read request {:?} with value {:x?}", &req, &value);
                    Ok(value)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}