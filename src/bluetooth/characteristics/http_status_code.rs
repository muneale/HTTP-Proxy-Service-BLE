use crate::AppState;
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicNotify, CharacteristicNotifyMethod};
use futures::FutureExt;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};
use crate::constants::HTTP_STATUS_CODE_UUID;

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    Characteristic {
        uuid: *HTTP_STATUS_CODE_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_status_code.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target: "http_status_code", "Read request {:?} with value {:x?}", &req, &value);
                    Ok(value)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                let value = state_w.http_status_code.clone();
                async move {
                    tokio::spawn(async move {
                        loop {
                            {
                                let value = value.lock().await;
                                debug!(target: "http_status_code", "Notifying with value {:x?}", &*value);
                                if let Err(err) = notifier.notify(value.to_vec()).await {
                                    warn!("Notification error: {}", &err);
                                    break;
                                }
                            }
                            sleep(Duration::from_secs(5)).await;
                        }
                    });
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}