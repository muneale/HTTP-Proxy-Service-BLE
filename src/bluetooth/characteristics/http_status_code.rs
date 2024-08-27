use crate::{constants::{EVENT_EMITTER, HTTP_STATUS_CODE_UPDATED_EVENT, HTTP_STATUS_CODE_UUID}, AppState};
use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicNotify, CharacteristicNotifyMethod};
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub fn create_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    let event_listeners_queue = Arc::new(Mutex::new(Vec::<String>::new()));
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
                let listeners = event_listeners_queue.clone();
                async move {
                    let mut listeners = listeners.lock().await;
                    if !listeners.is_empty() {
                        for listener in listeners.iter() {
                            debug!("Removing listener {}", listener);
                            EVENT_EMITTER.lock().await.remove_listener(listener);
                            debug!("Listener {} removed successfully", listener);
                        }
                    }
                    let notifier = notifier.clone();
                    debug!("Initializing event {}", HTTP_STATUS_CODE_UPDATED_EVENT);
                    let event = EVENT_EMITTER.lock().await.on(HTTP_STATUS_CODE_UPDATED_EVENT, move |value: Vec<u8>| {
                        debug!("Event {} triggered", HTTP_STATUS_CODE_UPDATED_EVENT);
                        let notifier = notifier.clone();
                        debug!("Notifying with value {:x?}", &*value);
                        futures::executor::block_on(async move {
                            let mut notifier = notifier.lock().await;
                            if let Err(err) = notifier.notify(value).await {
                                warn!("Notification error: {}", &err);
                                return;
                            }
                            debug!("Notification sent");
                        });
                    });
                    *listeners = vec![event.clone()];
                    debug!("Event {} with id {} intialized successfully", HTTP_STATUS_CODE_UPDATED_EVENT, event);       
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}