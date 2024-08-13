use crate::AppState;
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;
use crate::constants::HTTP_HEADERS_BODY_CHUNK_IDX_UUID;

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    Characteristic {
        uuid: *HTTP_HEADERS_BODY_CHUNK_IDX_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_headers_body_chunk_idx.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target: "headers_body_chunk_idx", "Read request {:?} with value {:x?}", &req, &value);
                    Ok(value)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                let value = state_w.http_headers_body_chunk_idx.clone();
                async move {
                    debug!(target: "headers_body_chunk_idx", "Write request {:?} with value {:x?}", &req, &new_value);
                    let mut value = value.lock().await;
                    *value = new_value;
                    Ok(())
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}