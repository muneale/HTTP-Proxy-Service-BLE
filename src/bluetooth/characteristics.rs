use crate::app_state::{AppState, SharedBuffer};
use crate::constants::*;
use crate::http_handler::handle_http_control_point;
use bluer::gatt::local::{Characteristic, CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod};
use futures::FutureExt;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

// Helper functions to create characteristics
fn create_http_headers_body_mtu_sizes_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state = state.clone();
    Characteristic {
        uuid: *HTTP_HEADERS_BODY_SIZES_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state.http_headers_body_sizes.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target="create_http_headers_body_mtu_sizes_characteristic", "Read request {:?} with value {:x?}", &req, &value);
                    Ok(value)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn create_http_headers_body_chunk_idx_characteristic(state: &Arc<AppState>) -> Characteristic {
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
                    debug!(target="create_http_headers_body_chunk_idx_characteristic", "Read request {:?} with value {:x?}", &req, &value);
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
                    debug!(target="create_http_headers_body_chunk_idx_characteristic", "Write request {:?} with value {:x?}", &req, &new_value);
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

fn create_http_uri_characteristic(state: &Arc<AppState>) -> Characteristic {
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
                    debug!(target="create_http_uri_characteristic", "Read request {:?} with value {:x?}", &req, &value);
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
                    debug!(target="create_http_uri_characteristic", "Write request {:?} with value {:x?}", &req, &new_value);
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

fn create_http_headers_characteristic(state: &Arc<AppState>, mtu: usize) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    Characteristic {
        uuid: *HTTP_HEADERS_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_headers.clone();
                let headers_idx = state_r.http_headers_body_chunk_idx.clone();
                let mtu = if mtu > 0 && mtu < req.mtu as usize { mtu } else { req.mtu as usize - MTU_OVERHEAD };
                async move {
                    let value = value.lock().await.clone();
                    let headers_idx = headers_idx.lock().await.clone();
                    let len = value.len();
                    if len <= mtu {
                        return Ok(value);
                    }
                    let idx = if headers_idx.len() >= 4 {
                        u32::from_le_bytes(headers_idx[0..4].try_into().unwrap())
                    } else {
                        0
                    } as usize;
                    let start = if (idx * mtu) < len { idx * mtu } else { len - (idx - 1) * mtu };
                    let end = if ((idx + 1) * mtu) < len { (idx + 1) * mtu } else { len };
                    let truncated_value = value[start..end].to_vec();
                    debug!(target="create_http_headers_characteristic", "Read request {:?} with value {:x?}", &req, &truncated_value);
                    Ok(truncated_value)
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
                    debug!(target="create_http_headers_characteristic", "Write request {:?} with value {:x?}", &req, &new_value);
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

fn create_http_status_code_characteristic(state: &Arc<AppState>) -> Characteristic {
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
                    debug!(target="create_http_status_code_characteristic", "Read request {:?} with value {:x?}", &req, &value);
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
                        debug!(
                            target="create_http_status_code_characteristic",
                            "Notification session start with confirming={:?}",
                            notifier.confirming()
                        );
                        loop {
                            {
                                let value = value.lock().await;
                                debug!(target="create_http_status_code_characteristic", "Notifying with value {:x?}", &*value);
                                if let Err(err) = notifier.notify(value.to_vec()).await {
                                    warn!("Notification error: {}", &err);
                                    break;
                                }
                            }
                            sleep(Duration::from_secs(5)).await;
                        }
                        debug!(target="create_http_status_code_characteristic", "Notification session stop");
                    });
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn create_http_entity_body_characteristic(state: &Arc<AppState>, mtu: usize) -> Characteristic {
    let state_r = state.clone();
    let state_w = state.clone();
    Characteristic {
        uuid: *HTTP_ENTITY_BODY_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.http_entity_body.clone();
                let body_idx = state_r.http_headers_body_chunk_idx.clone();
                let mtu = if mtu > 0 && mtu < req.mtu as usize { mtu } else { req.mtu as usize - MTU_OVERHEAD };
                async move {
                    let value = value.lock().await.clone();
                    let body_idx = body_idx.lock().await.clone();
                    let len = value.len();
                    if len <= mtu {
                        return Ok(value);
                    }
                    let idx = if body_idx.len() >= 8 {
                        u32::from_le_bytes(body_idx[4..8].try_into().unwrap())
                    } else {
                        0
                    } as usize;
                    let start = if (idx * mtu) < len { idx * mtu } else { len - (idx - 1) * mtu };
                    let end = if ((idx + 1) * mtu) < len { (idx + 1) * mtu } else { len };
                    let truncated_value = value[start..end].to_vec();
                    debug!(target="create_http_entity_body_characteristic", "Read request {:?} with value {:x?}", &req, &truncated_value);
                    Ok(truncated_value)
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
                    debug!(target="create_http_entity_body_characteristic", "Write request {:?} with value {:x?}", &req, &new_value);
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

fn create_https_security_characteristic(state: &Arc<AppState>) -> Characteristic {
    let state_r = state.clone();
    Characteristic {
        uuid: *HTTPS_SECURITY_UUID,
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |req| {
                let value = state_r.https_security.clone();
                async move {
                    let value = value.lock().await.clone();
                    debug!(target="create_https_security_characteristic", "Read request {:?} with value {:x?}", &req, &value);
                    Ok(value)
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn create_http_control_point_characteristic(state: &Arc<AppState>, timeout: u64, mtu: usize) -> Characteristic {
    let state_r = state.clone();
    Characteristic {
        uuid: *HTTP_CONTROL_POINT_UUID,
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                let state = state_r.clone();
                async move {
                    let _ = handle_http_control_point(&state, new_value, req, timeout, mtu).await;
                    Ok(())
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}