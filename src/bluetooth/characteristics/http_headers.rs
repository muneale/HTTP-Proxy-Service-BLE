use crate::{AppState, Config, constants::HTTP_HEADERS_UUID, utils};
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;

pub fn create_characteristic(state: &Arc<AppState>, config: &Config) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    let config = config.clone();
    Characteristic {
        uuid: *HTTP_HEADERS_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_headers.clone();
                let headers_idx = state_r.http_headers_body_chunk_idx.clone();
                let effective_mtu = config.effective_mtu(req.mtu as usize);
                async move {
                    let value = value.lock().await;
                    let headers_idx = headers_idx.lock().await;
                    
                    let chunk_index = utils::get_chunk_index(&headers_idx, true).unwrap();
                    let total_len = value.len();
                    
                    let start = chunk_index * effective_mtu;
                    let end = (start + effective_mtu).min(total_len);
                    
                    let chunk = if start < total_len {
                        value[start..end].to_vec()
                    } else {
                        Vec::new()
                    };
                    
                    debug!(target: "http_headers", "Read request {:?} with chunk {:x?} (index: {}, start: {}, end: {})", &req, &chunk, chunk_index, start, end);
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
                let value = state_w.http_headers.clone();
                async move {
                    debug!(target: "http_headers", "Write request {:?} with value {:x?}", &req, &new_value);
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