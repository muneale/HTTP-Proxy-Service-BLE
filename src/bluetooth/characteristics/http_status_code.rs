use crate::{constants::{EVENT_EMITTER, HTTP_STATUS_CODE_UPDATED_EVENT, HTTP_STATUS_CODE_UUID}, AppState};
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicNotify, CharacteristicNotifyMethod};
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
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
            method: CharacteristicNotifyMethod::Fun(Box::new(move |notifier| {
                let notifier = Arc::new(Mutex::new(notifier));
                async move {
                   let listeners = EVENT_EMITTER.lock().await.listeners.len();
                   debug!("Listeners for event {}: {}", HTTP_STATUS_CODE_UPDATED_EVENT, listeners);
                   if listeners == 0 {
                        debug!("Initializing event {}", HTTP_STATUS_CODE_UPDATED_EVENT);
                        EVENT_EMITTER.lock().await.on(HTTP_STATUS_CODE_UPDATED_EVENT, move |value: Vec<u8>| {
                            debug!("Event {} triggered", HTTP_STATUS_CODE_UPDATED_EVENT);
                            let notifier = notifier.clone();
                            debug!("Notifying with value {:x?}", &*value);
                            let _ = notifier.lock().then(|mut notifier| async move {
                                if let Err(err) = notifier.notify(value).await {
                                    warn!("Notification error: {}", &err);
                                }
                            });
                        });
                        debug!("Event {} intialized successfully", HTTP_STATUS_CODE_UPDATED_EVENT);       
                   }
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}