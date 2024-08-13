use crate::AppState;
use bluer::gatt::local::{Characteristic, CharacteristicRead};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;
use crate::constants::HTTPS_SECURITY_UUID;

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    Characteristic {
        uuid: *HTTPS_SECURITY_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.https_security.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target: "https_security", "Read request {:?} with value {:x?}", &req, &value);
                    Ok(value)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}