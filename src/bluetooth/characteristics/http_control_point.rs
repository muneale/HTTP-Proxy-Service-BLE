use crate::{http, AppState, Config};
use bluer::gatt::local::{Characteristic, CharacteristicWrite, CharacteristicWriteMethod};
use futures::FutureExt;
use std::sync::Arc;
use tracing::debug;
use crate::constants::HTTP_CONTROL_POINT_UUID;

pub fn create_characteristic(state: &Arc<AppState>, config: &Config) -> Characteristic {
    let state_r = state.clone();
    let config = config.clone();
    Characteristic {
        uuid: *HTTP_CONTROL_POINT_UUID,
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                let state = state_r.clone();
                let config = config.clone();
                async move {
                    debug!(target: "http_control_point", "Write request {:?} with value {:x?}", &req, &new_value);
                    let mtu = config.effective_mtu(req.mtu as usize);
                    let timeout = config.timeout_duration();
                    let _ = http::handler::handle_http_control_point(
                        &state,
                        new_value,
                        req,
                        timeout,
                        mtu
                    ).await;
                    Ok(())
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

