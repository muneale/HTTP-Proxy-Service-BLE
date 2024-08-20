use crate::{AppState, constants::HTTP_STATUS_CODE_UUID};
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicNotify, CharacteristicNotifyMethod};
use futures::FutureExt;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    let state_n = state.clone();
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
                let value = state_n.http_status_code.clone();
                async move {
                    tokio::spawn(async move {
                        let mut previous_value = value.lock().await.clone();
                        loop {
                            {
                                let value = value.lock().await.to_vec();
                                debug!(target: "http_status_code", "Notifying with value {:x?} and previous value {:x?}", &*value, &previous_value);
                                if previous_value == value {
                                    debug!(target: "http_status_code", "Previous and current value are the same, skipping notification.");
                                    break;
                                }
                                previous_value = value.clone();
                                if let Err(err) = notifier.notify(value).await {
                                    warn!("Notification error: {}", &err);
                                    break;
                                }
                                // previous_value = value.clone();
                            }
                            sleep(Duration::from_secs(1)).await;
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