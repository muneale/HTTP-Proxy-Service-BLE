//! Serves a Bluetooth GATT HPS server.

use bluer::{
    adv::Advertisement,
    gatt::local::{
        Application, Characteristic, CharacteristicNotify, CharacteristicNotifyMethod,
        CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod, Service,
    },
    UuidExt,
};
use byteorder::{LittleEndian, WriteBytesExt};
use clap::Parser;
use futures::FutureExt;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use reqwest::Method;
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::Mutex,
    time::sleep,
};
use tracing::{debug, error, info, warn};

// Constants
const MTU_OVERHEAD: usize = 3;
static SERVICE_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x1823));
static HTTP_URI_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB6));
static HTTP_HEADERS_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB7));
static HTTP_STATUS_CODE_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB8));
static HTTP_ENTITY_BODY_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AB9));
static HTTP_CONTROL_POINT_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2ABA));
static HTTPS_SECURITY_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2ABB));
static HTTP_HEADERS_BODY_CHUNK_IDX_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2A9A));
static HTTP_HEADERS_BODY_SIZES_UUID: Lazy<uuid::Uuid> = Lazy::new(|| uuid::Uuid::from_u16(0x2AC0));
// Type aliases
type SharedBuffer = Arc<Mutex<Vec<u8>>>;

#[derive(Clone, Debug, Copy, FromPrimitive)]
#[repr(u8)]
enum HttpControlOption {
    Invalid = 0,
    Get = 1,
    Head = 2,
    Post = 3,
    Put = 4,
    Delete = 5,
    SecureGet = 6,
    SecureHead = 7,
    SecurePost = 8,
    SecurePut = 9,
    SecureDelete = 10,
    Cancel = 11,
}

#[derive(Clone, Debug, Copy)]
#[repr(u8)]
enum HttpDataStatusBit {
    HeadersReceived = 1,
    HeadersTruncated = 2,
    BodyReceived = 4,
    BodyTruncated = 8,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "Logbot-HPS", help = "Service name")]
    name: String,
    #[arg(short, long, default_value = "60", help = "HTTP requests timeout in seconds")]
    timeout: u64,
    #[arg(short, long, default_value = "0", help = "Overrides the MTU size in bytes. When set to 0, it uses the established MTU size between client and server. Ignored when the value is greater than established MTU size.")]
    mtu: usize,
}

#[derive(Error, Debug)]
enum AppError {
    #[error("Bluetooth error: {0}")]
    BluetoothError(#[from] bluer::Error),
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

struct AppState {
    http_uri: SharedBuffer,
    http_headers: SharedBuffer,
    http_status_code: SharedBuffer,
    http_entity_body: SharedBuffer,
    https_security: SharedBuffer,
    http_headers_body_chunk_idx: SharedBuffer,
    http_headers_body_sizes: SharedBuffer,
}

impl AppState {
    fn new() -> Self {
        Self {
            http_uri: Arc::new(Mutex::new(Vec::new())),
            http_headers: Arc::new(Mutex::new(Vec::new())),
            http_status_code: Arc::new(Mutex::new(Vec::new())),
            http_entity_body: Arc::new(Mutex::new(Vec::new())),
            https_security: Arc::new(Mutex::new(Vec::new())),
            http_headers_body_chunk_idx: Arc::new(Mutex::new(vec![0; 8])),
            http_headers_body_sizes: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!(target="main", "Service started with arguments: {:?}", args);

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    info!(
        target="main",
        "Advertising on Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );

    let le_advertisement = Advertisement {
        service_uuids: vec![*SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some(args.name.to_string()),
        ..Default::default()
    };
    let adv_handle = adapter.advertise(le_advertisement).await?;

    info!(
        target="main",
        "Serving GATT echo service on Bluetooth adapter {}",
        adapter.name()
    );

    let state = Arc::new(AppState::new());

    let app = create_application(&state, args.timeout, args.mtu);
    let app_handle = adapter.serve_gatt_application(app).await?;

    info!(target="main", "Service ready.");

    handle_signals().await?;

    info!(target="main", "Removing service and advertisement");
    drop(app_handle);
    drop(adv_handle);
    sleep(Duration::from_secs(1)).await;

    Ok(())
}

fn create_application(state: &Arc<AppState>, timeout: u64, mtu: usize) -> Application {
    Application {
        services: vec![Service {
            uuid: *SERVICE_UUID,
            primary: true,
            characteristics: vec![
                create_http_headers_body_mtu_sizes_characteristic(state),
                create_http_headers_body_chunk_idx_characteristic(state),
                create_http_uri_characteristic(state),
                create_http_headers_characteristic(state, mtu),
                create_http_status_code_characteristic(state),
                create_http_entity_body_characteristic(state, mtu),
                create_https_security_characteristic(state),
                create_http_control_point_characteristic(state, timeout, mtu),
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}

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

async fn handle_http_control_point(
    state: &Arc<AppState>,
    new_value: Vec<u8>,
    req: bluer::gatt::local::CharacteristicWriteRequest,
    timeout: u64,
    mtu: usize
) -> Result<(), AppError> {
    // Method and protocol
    let (method, protocol) = match new_value.first() {
        Some(&first) => match HttpControlOption::from_u8(first) {
            Some(HttpControlOption::Get) => (Method::GET, "http"),
            Some(HttpControlOption::Head) => (Method::HEAD, "http"),
            Some(HttpControlOption::Post) => (Method::POST, "http"),
            Some(HttpControlOption::Put) => (Method::PUT, "http"),
            Some(HttpControlOption::Delete) => (Method::DELETE, "http"),
            Some(HttpControlOption::SecureGet) => (Method::GET, "https"),
            Some(HttpControlOption::SecureHead) => (Method::HEAD, "https"),
            Some(HttpControlOption::SecurePost) => (Method::POST, "https"),
            Some(HttpControlOption::SecurePut) => (Method::PUT, "https"),
            Some(HttpControlOption::SecureDelete) => (Method::DELETE, "https"),
            Some(HttpControlOption::Cancel) => {
                debug!(target="handle_http_control_point", "Request cancelled");
                return Ok(());
            }
            _ => {
                error!(target="handle_http_control_point", "Invalid method");
                return Ok(());
            }
        },
        None => {
            error!(target="handle_http_control_point", "No method provided");
            return Ok(());
        }
    };

    debug!(target="handle_http_control_point", "Method: '{}', Protocol: '{}'", method, protocol);

    // URL
    let address = String::from_utf8(state.http_uri.lock().await.clone())?;
    if address.is_empty() {
        error!(target="handle_http_control_point", "No URL provided");
        return Ok(());
    }
    let url = format!("{}://{}", protocol, address);
    debug!(target="handle_http_control_point", "Sending request to '{}'", url);

    // Headers
    let headers_str = String::from_utf8(state.http_headers.lock().await.clone())?;
    let client = reqwest::Client::new();
    let mut req_builder = client
        .request(method, url)
        .timeout(Duration::from_secs(timeout));

    for h in headers_str.split("\r\n") {
        if let Some(i) = h.find(':') {
            let (header_key, header_value) = h.split_at(i);
            let header_key = header_key.trim();
            let header_value = header_value[1..].trim(); // Skip the ':' and trim
            debug!(target="handle_http_control_point", "Header: '{}: {}'", header_key, header_value);
            req_builder = req_builder.header(header_key, header_value);
        }
    }

    // Body
    let body = String::from_utf8(state.http_entity_body.lock().await.clone())?;
    debug!(target="handle_http_control_point", "Body: '{}'", body);
    if !body.is_empty() {
        req_builder = req_builder.body(body);
    }

    // Send request and handle response
    let res = req_builder.send().await?;
    debug!(target="handle_http_control_point", "Response: {:?}", &res);

    let mut status = Vec::new();
    status.write_u16::<LittleEndian>(res.status().as_u16())?;

    // Write headers into buffer
    let headers_str = res
        .headers()
        .iter()
        .map(|(k, v)| format!("{}: {}\r\n", k.as_str(), v.to_str().unwrap_or("")))
        .collect::<String>();

    let mut header_values = state.http_headers.lock().await;
    *header_values = headers_str.into_bytes();

    let mtu = if mtu > 0 && mtu < req.mtu as usize { mtu } else { req.mtu as usize - MTU_OVERHEAD };
    let headers_status = if header_values.len() <= mtu {
        HttpDataStatusBit::HeadersReceived as u8
    } else {
        HttpDataStatusBit::HeadersTruncated as u8
    };

    // Write body into buffer
    let body_bytes = res.bytes().await?;
    let mut body_values = state.http_entity_body.lock().await;
    *body_values = body_bytes.to_vec();

    // Set headers, body and MTU sizes
    let mut headers_body_sizes = Vec::new();
    
    headers_body_sizes.write_u32::<LittleEndian>(header_values.len() as u32)?;
    headers_body_sizes.write_u32::<LittleEndian>(body_values.len() as u32)?;
    headers_body_sizes.write_u32::<LittleEndian>(mtu as u32)?;
    let mut byte_headers_body_sizes_values = state.http_headers_body_sizes.lock().await;
    *byte_headers_body_sizes_values = headers_body_sizes;

    // Set chunk indexes to 0
    let chunk_idxs_values = vec![0; 8];
    let mut chunk_idxs = state.http_headers_body_chunk_idx.lock().await;
    *chunk_idxs = chunk_idxs_values;

    let body_status = if body_values.len() <= mtu {
        HttpDataStatusBit::BodyReceived as u8
    } else {
        HttpDataStatusBit::BodyTruncated as u8
    };
    status.push(headers_status | body_status);

    // Write HTTP response code
    let mut status_values = state.http_status_code.lock().await;
    *status_values = status;

    debug!(target="handle_http_control_point", "Write request {:?} completed", &req);

    Ok(())
}

async fn handle_signals() -> Result<(), AppError> {
    let mut signal_terminate = signal(SignalKind::terminate())?;
    let mut signal_interrupt = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = signal_terminate.recv() => info!(target="handle_signals", "Received SIGTERM"),
        _ = signal_interrupt.recv() => info!(target="handle_signals", "Received SIGINT"),
    };
    Ok(())
}