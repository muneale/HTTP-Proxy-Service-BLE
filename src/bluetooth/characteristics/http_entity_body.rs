use crate::{AppState, Config, utils};
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;
use crate::constants::HTTP_ENTITY_BODY_UUID;

pub fn create_characteristic(state: &Arc<AppState>, config: &Config) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    let config = config.clone();
    Characteristic {
        uuid: *HTTP_ENTITY_BODY_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_entity_body.clone();
                let body_idx = state_r.http_headers_body_chunk_idx.clone();
                let effective_mtu = config.effective_mtu(req.mtu as usize);
                async move {
                    let value = value.lock().await;
                    let body_idx = body_idx.lock().await;
                    
                    let chunk_index = utils::get_chunk_index(&body_idx, false).unwrap();
                    let total_len = value.len();
                    
                    let start = chunk_index * effective_mtu;
                    let end = (start + effective_mtu).min(total_len);
                    
                    let chunk = if start < total_len {
                        value[start..end].to_vec()
                    } else {
                        Vec::new()
                    };
                    
                    debug!(target: "http_entity_body", "Read request {:?} with chunk {:x?} (index: {}, start: {}, end: {})", &req, &chunk, chunk_index, start, end);
                    Ok(chunk)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                let value = state_w.http_entity_body.clone();
                async move {
                    debug!(target: "http_entity_body", "Write request {:?} with value {:x?}", &req, &new_value);
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