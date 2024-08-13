use crate::{AppState, constants::MTU_OVERHEAD, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use reqwest::Method;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

#[derive(Clone, Debug, Copy, FromPrimitive)]
#[repr(u8)]
pub enum HttpControlOption {
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
pub enum HttpDataStatusBit {
    HeadersReceived = 1,
    HeadersTruncated = 2,
    BodyReceived = 4,
    BodyTruncated = 8,
}

pub async fn handle_http_control_point(
    state: &Arc<AppState>,
    new_value: Vec<u8>,
    req: bluer::gatt::local::CharacteristicWriteRequest,
    timeout: Duration,
    mtu: usize
) -> Result<()> {
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
        .timeout(timeout);

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