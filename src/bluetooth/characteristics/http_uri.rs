use crate::AppState;
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;
use crate::constants::HTTP_URI_UUID;

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    Characteristic {
        uuid: *HTTP_URI_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_uri.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target: "http_uri", "Read request {:?} with value {:x?}", &req, &value);
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
                let value = state_w.http_uri.clone();
                async move {
                    debug!(target: "http_uri", "Write request {:?} with value {:x?}", &req, &new_value);
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