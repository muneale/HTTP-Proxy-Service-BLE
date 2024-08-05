//! Serves a Bluetooth GATT HPS server.

use bluer::{
    adv::Advertisement,
    gatt::local::{
        Application,
        Characteristic,
        CharacteristicNotify,
        CharacteristicNotifyMethod,
        CharacteristicRead,
        CharacteristicWrite,
        CharacteristicWriteMethod,
        Service
    },
    UuidExt,
};
use byteorder::{LittleEndian, WriteBytesExt};
use futures::FutureExt;
use log::{debug, error, info};
use reqwest::{Method, Response};
use std::{env, sync::Arc, time::Duration};
use substring::Substring;
use tokio::{
    sync::Mutex,
    time::sleep,
    signal::unix::{signal, SignalKind}
};


#[derive(Clone, Debug, Copy)]
#[repr(u8)]
enum HttpControlOption {
    Invalid = 0,
    Get = 1, // HTTP GET Request	N/A	Initiates an HTTP GET Request.
    Head = 2, //	HTTP HEAD Request	N/A	Initiates an HTTP HEAD Request.
    Post = 3, //	HTTP POST Request	N/A	Initiates an HTTP POST Request.
    Put = 4, //	HTTP PUT Request	N/A	Initiates an HTTP PUT Request.
    Delete = 5, //	HTTP DELETE Request	N/A	Initiates an HTTP DELETE Request.
    SecureGet = 6, //	HTTPS GET Request	N/A	Initiates an HTTPS GET Reques.t
    SecureHead = 7, //	HTTPS HEAD Request	N/A	Initiates an HTTPS HEAD Request.
    SecurePost = 8, //	HTTPS POST Request	N/A	Initiates an HTTPS POST Request.
    SecurePut = 9, //	HTTPS PUT Request	N/A	Initiates an HTTPS PUT Request.
    SecureDelete = 10, //	HTTPS DELETE Request	N/A	Initiates an HTTPS DELETE Request.
    Cancel = 11, //	HTTP Request Cancel	N/A	Terminates any executing HTTP Request from the HPS Client.
}

impl HttpControlOption {
    pub fn from_u8(i: u8) -> HttpControlOption {
        match i {
            1 => HttpControlOption::Get,
            2 => HttpControlOption::Head,
            3 => HttpControlOption::Post,
            4 => HttpControlOption::Put,
            5 => HttpControlOption::Delete,
            6 => HttpControlOption::SecureGet,
            7 => HttpControlOption::SecureHead,
            8 => HttpControlOption::SecurePost,
            9 => HttpControlOption::SecurePut,
            10 => HttpControlOption::SecureDelete,
            11 => HttpControlOption::Cancel,
            _ => HttpControlOption::Invalid,
        }
    }
}

#[derive(Clone, Debug, Copy)]
#[repr(u8)]
enum HttpDataStatusBit {
    // 3rd byte of http_status_code
    HeadersReceived = 1, // Headers Received
    // 0	The response-header and entity-header fields were not received in the HTTP response or stored in the HTTP Headers characteristic.
    // 1	The response-header and entity-header fields were received in the HTTP response and stored in the HTTP Headers characteristic for the Client to read.
    HeadersTruncated = 2, // Headers Truncated
    // 0	Any received response-header and entity-header fields did not exceed 512 octets in length.
    // 1	The response-header and entity-header fields exceeded 512 octets in length and the first 512 octets were saved in the HTTP Headers characteristic.
    BodyReceived = 4, // Body Received
    // 0	The entity-body field was not received in the HTTP response or stored in the HTTP Entity Body characteristic.
    // 1	The entity-body field was received in the HTTP response and stored in the HTTP Entity Body characteristic for the Client to read.
    BodyTruncated = 8, // Body Truncated
    // 0	Any received entity-body field did not exceed 512 octets in length.
    // 1	The entity-body field exceeded 512 octets in length and the first 512 octets were saved in the HTTP Headers characteristic
}


#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();

    // let cmd = clap::Command::new("cargo")
    //     .bin_name("cargo")
    //     .subcommand_required(true)
    //     .subcommand(
    //     clap::command!("example").arg(
    //         clap::arg!(--"manifest-path" <PATH>)
    //             .value_parser(clap::value_parser!(std::path::PathBuf)),
    //     ),
    // );
    // let matches = cmd.get_matches();

    // return Ok(());

    let service_uuid = uuid::Uuid::from_u16(0x1823); // HTTP Proxy Service
    let http_uri_uuid = uuid::Uuid::from_u16(0x2AB6);
    let http_headers_uuid = uuid::Uuid::from_u16(0x2AB7);
    let http_status_code_uuid = uuid::Uuid::from_u16(0x2AB8);
    let http_entity_body_uuid = uuid::Uuid::from_u16(0x2AB9);
    let http_control_point_uuid = uuid::Uuid::from_u16(0x2ABA);
    let https_security_uuid = uuid::Uuid::from_u16(0x2ABB);


    let session: bluer::Session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    info!("Advertising on Bluetooth adapter {} with address {}", adapter.name(), adapter.address().await?);
    let le_advertisement = Advertisement {
        service_uuids: vec![service_uuid].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("gatt_hps_server".to_string()),
        ..Default::default()
    };
    let adv_handle = adapter.advertise(le_advertisement).await?;

    info!("Serving GATT echo service on Bluetooth adapter {}", adapter.name());
    
    let http_uri: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let http_uri_read = http_uri.clone();
    let http_uri_write = http_uri.clone();

    let http_headers: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let http_headers_read = http_headers.clone();
    let http_headers_write = http_headers.clone();

    let http_status_code: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let http_status_code_read = http_status_code.clone();
    let http_status_code_notify = http_status_code.clone();

    let http_entity_body: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let http_entity_body_read = http_entity_body.clone();
    let http_entity_body_write = http_entity_body.clone();

    let https_security: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let https_security_read = https_security.clone();

    let http_control_point_uri = http_uri.clone();
    let http_control_point_headers = http_headers.clone();
    let http_control_point_status_code = http_status_code.clone();
    let http_control_point_entity_body = http_entity_body.clone();
    let http_control_point_security = https_security.clone();
    
    let app = Application {
        services: vec![Service {
            uuid: service_uuid,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: http_uri_uuid,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            let value = http_uri_read.clone();
                            async move {
                                let value = value.lock().await.clone();
                                debug!("Read request {:?} with value {:x?}", &req, &value);
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
                            let value = http_uri_write.clone();
                            async move {
                                debug!("Write request {:?} with value {:x?}", &req, &new_value);
                                let mut value = value.lock().await;
                                *value = new_value;
                                Ok(())
                            }
                            .boxed()
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: http_headers_uuid,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            let value = http_headers_read.clone();
                            async move {
                                let value = value.lock().await.clone();
                                debug!("Read request {:?} with value {:x?}", &req, &value);
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
                            let value = http_headers_write.clone();
                            async move {
                                debug!("Write request {:?} with value {:x?}", &req, &new_value);
                                let mut value = value.lock().await;
                                *value = new_value;
                                Ok(())
                            }
                            .boxed()
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: http_status_code_uuid,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            let value = http_status_code_read.clone();
                            async move {
                                let value = value.lock().await.clone();
                                debug!("Read request {:?} with value {:x?}", &req, &value);
                                Ok(value)
                            }
                            .boxed()
                        }),
                        ..Default::default()
                    }),
                    notify: Some(CharacteristicNotify {
                        notify: true,
                        method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                            let value = http_status_code_notify.clone();
                            async move {
                                tokio::spawn(async move {
                                    debug!(
                                        "Notification session start with confirming={:?}",
                                        notifier.confirming()
                                    );
                                    loop {
                                        {
                                            let value = value.lock().await;
                                            debug!("Notifying with value {:x?}", &*value);
                                            if let Err(err) = notifier.notify(value.to_vec()).await {
                                                error!("Notification error: {}", &err);
                                                break;
                                            }
                                        }
                                        sleep(Duration::from_secs(5)).await;
                                    }
                                    debug!("Notification session stop");
                                });
                            }
                            .boxed()
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: http_entity_body_uuid,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            let value = http_entity_body_read.clone();
                            async move {
                                let value = value.lock().await.clone();
                                debug!("Read request {:?} with value {:x?}", &req, &value);
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
                            let value = http_entity_body_write.clone();
                            async move {
                                debug!("Write request {:?} with value {:x?}", &req, &new_value);
                                let mut value = value.lock().await;
                                *value = new_value;
                                Ok(())
                            }
                            .boxed()
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: https_security_uuid,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            let value = https_security_read.clone();
                            async move {
                                let value = value.lock().await.clone();
                                debug!("Read request {:?} with value {:x?}", &req, &value);
                                Ok(value)
                            }
                            .boxed()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: http_control_point_uuid,
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: true,
                        method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                            let byte_uri = http_control_point_uri.clone();
                            let byte_headers = http_control_point_headers.clone();
                            let byte_body = http_control_point_entity_body.clone();
                            let byte_status = http_control_point_status_code.clone();
                            async move {
                                // Method and protocol
                                let method: Method;
                                let protocol: &str;
                                match new_value.first() {
                                    Some(first) => {
                                        match HttpControlOption::from_u8(*first) {
                                            HttpControlOption::Get => { method = Method::GET; protocol = "http"; },
                                            HttpControlOption::Head => { method = Method::HEAD; protocol = "http"; },
                                            HttpControlOption::Post => { method = Method::POST; protocol = "http"; },
                                            HttpControlOption::Put => { method = Method::PUT; protocol = "http"; },
                                            HttpControlOption::Delete => { method = Method::DELETE; protocol = "http"; },
                                            HttpControlOption::SecureGet => { method = Method::GET; protocol = "https"; },
                                            HttpControlOption::SecureHead => { method = Method::HEAD; protocol = "https"; },
                                            HttpControlOption::SecurePost => { method = Method::POST; protocol = "https"; },
                                            HttpControlOption::SecurePut => { method = Method::PUT; protocol = "https"; },
                                            HttpControlOption::SecureDelete => { method = Method::DELETE; protocol = "https"; },
                                            _ => { 
                                                error!("Invalid method");
                                                return Ok(());
                                            },
                                        }
                                    },
                                    None => {
                                        error!("No number has been provided");
                                        return Ok(());
                                    },
                                }
                                debug!("Method: '{}', Protocol: '{}'", method, protocol);

                                // URL
                                let address = match String::from_utf8(byte_uri.lock().await.to_vec()) {
                                    Ok(string) => string,
                                    Err(e) => {
                                        error!("Unable to parse uri as string. Reason: {}", e);
                                        String::new()
                                    },
                                };
                                if address == "" {
                                    return Ok(())
                                }
                                let url = format!("{}://{}", protocol, address);
                                debug!("Sending request to '{}'", url);

                                // Headers
                                let headers: String;
                                match String::from_utf8(byte_headers.lock().await.to_vec()) {
                                    Ok(s) => headers = s,
                                    Err(e) => {
                                        error!("Unable to parse headers as string. Reason: {}", e);
                                        return Ok(());
                                    },
                                };

                                let client = reqwest::Client::new();
                                let mut req_builder = client.request(method, url).timeout(Duration::new(60, 0));
                                for h in headers.split("\r\n") {
                                    let i = match h.find(":") {
                                        Some(k) => k.try_into().unwrap(),
                                        None => continue,
                                    };
                                    let header_key = h.substring(0, i).trim().to_string();
                                    let header_value = h.substring(i+1, h.len()).trim().to_string();
                                    debug!("Header: '{}: {}'", header_key, header_value);
                                    req_builder = req_builder.header(header_key, header_value);
                                }

                                // Body
                                let body: String;
                                match String::from_utf8(byte_body.lock().await.to_vec()) {
                                    Ok(s) => body = s,
                                    Err(e) => {
                                        error!("Unable to parse body as string. Reason: {}", e);
                                        return Ok(());
                                    },
                                };
                                debug!("Body: '{}'", body);
                                if body != "" {
                                    req_builder = req_builder.body(body);
                                }

                                // Response
                                let res: Response;
                                match req_builder.send().await {
                                    Ok(r) => res = r,
                                    Err(e) => {
                                        error!("Unable to send the request. Reason: {}", e);
                                        return Ok(());
                                    }
                                };
                                debug!("Response: {:?}", &res);

                                let mut status = Vec::new();
                                status.write_u16::<LittleEndian>(res.status().into()).unwrap();

                                // Write headers into buffer
                                let mut headers_str = String::new();
                                for (k, v) in res.headers() {
                                    headers_str = format!("{}{}: {}\r\n", headers_str, k.as_str().to_owned(), String::from_utf8_lossy(v.as_bytes()).into_owned());
                                }
                                let mut header_values = byte_headers.lock().await;
                                *header_values = headers_str.as_bytes().to_vec();
                                let headers_status = if header_values.len() <= req.mtu.into() { HttpDataStatusBit::HeadersReceived as u8 } else { HttpDataStatusBit::HeadersTruncated as u8 };

                                // Write body into buffer
                                let mut body_values = byte_body.lock().await;
                                *body_values = match &res.bytes().await {
                                    Ok(b) => b.to_vec(),
                                    Err(e) => {
                                        error!("Unable to parse response body. Reason: {}", e);
                                        return Ok(());
                                    } 
                                };
                                let body_status = if body_values.len() <= req.mtu.into() { HttpDataStatusBit::BodyReceived as u8 } else { HttpDataStatusBit::BodyTruncated as u8 };
                                status.write_u8(headers_status | body_status).unwrap();
                                
                                // Write HTTP response code
                                let mut status_values = byte_status.lock().await;
                                *status_values = status;

                                debug!("Write request {:?} with value {:x?}", &req, &new_value);

                                Ok(())
                            }.boxed()
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    let app_handle = adapter.serve_gatt_application(app).await?;

    info!("Service ready.");

    // Graceful shutdown when sigint or sigterm are received
    let mut signal_terminate = signal(SignalKind::terminate())?;
    let mut signal_interrupt = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = signal_terminate.recv() => info!("Received SIGTERM"),
        _ = signal_interrupt.recv() => info!("Received SIGINT"),
    };

    info!("Removing service and advertisement");
    drop(app_handle);
    drop(adv_handle);
    sleep(Duration::from_secs(1)).await;
    
    Ok(())
}