// Copyright 2024-2025 Tree xie.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{get_hostname, Ctx};
use bytes::BytesMut;
use http::{HeaderName, HeaderValue};
use pingora::http::RequestHeader;
use pingora::proxy::Session;

pub const HOST_NAME_TAG: &[u8] = b"$hostname";
const HOST_TAG: &[u8] = b"$host";
const SCHEME_TAG: &[u8] = b"$scheme";
const REMOTE_ADDR_TAG: &[u8] = b"$remote_addr";
const REMOTE_PORT_TAG: &[u8] = b"$remote_port";
const SERVER_ADDR_TAG: &[u8] = b"$server_addr";
const SERVER_PORT_TAG: &[u8] = b"$server_port";
const PROXY_ADD_FORWARDED_TAG: &[u8] = b"$proxy_add_x_forwarded_for";
const UPSTREAM_ADDR_TAG: &[u8] = b"$upstream_addr";

static SCHEME_HTTPS: HeaderValue = HeaderValue::from_static("https");
static SCHEME_HTTP: HeaderValue = HeaderValue::from_static("http");

/// Get request host in this order of precedence:
/// host name from the request line,
/// or host name from the "Host" request header field
fn get_host(header: &RequestHeader) -> Option<&str> {
    if let Some(host) = header.uri.host() {
        return Some(host);
    }
    if let Some(host) = header.headers.get("Host") {
        if let Ok(value) = host.to_str().map(|host| host.split(':').next()) {
            return value;
        }
    }
    None
}

/// Processes special header values that contain dynamic variables.
/// Supports variables like $host, $scheme, $remote_addr etc.
///
/// # Arguments
/// * `value` - The header value to process
/// * `session` - The HTTP session context
/// * `ctx` - The application state
///
/// # Returns
/// * `Option<HeaderValue>` - The processed header value or None if no special handling needed
#[inline]
pub fn convert_header_value(
    value: &HeaderValue,
    session: &Session,
    ctx: &Ctx,
) -> Option<HeaderValue> {
    let buf = value.as_bytes();

    // Early return if not a special header (moved this check earlier)
    if buf.is_empty() || !(buf[0] == b'$' || buf[0] == b':') {
        return None;
    }

    // Helper closure to convert string to HeaderValue
    let to_header_value = |s: &str| HeaderValue::from_str(s).ok();

    match buf {
        HOST_TAG => get_host(session.req_header()).and_then(to_header_value),
        SCHEME_TAG => Some(if ctx.tls_version.is_some() {
            SCHEME_HTTPS.clone()
        } else {
            SCHEME_HTTP.clone()
        }),
        HOST_NAME_TAG => to_header_value(get_hostname()),
        REMOTE_ADDR_TAG => ctx.remote_addr.as_deref().and_then(to_header_value),
        REMOTE_PORT_TAG => ctx
            .remote_port
            .map(|p| p.to_string())
            .and_then(|s| to_header_value(&s)),
        SERVER_ADDR_TAG => ctx.server_addr.as_deref().and_then(to_header_value),
        SERVER_PORT_TAG => ctx
            .server_port
            .map(|p| p.to_string())
            .and_then(|s| to_header_value(&s)),
        UPSTREAM_ADDR_TAG => {
            if !ctx.upstream_address.is_empty() {
                to_header_value(&ctx.upstream_address)
            } else {
                None
            }
        },
        PROXY_ADD_FORWARDED_TAG => {
            ctx.remote_addr.as_deref().and_then(|remote_addr| {
                let value = match session
                    .get_header(HeaderName::from_static("X-Forwarded-For"))
                {
                    Some(existing) => format!(
                        "{}, {}",
                        existing.to_str().unwrap_or_default(),
                        remote_addr
                    ),
                    None => remote_addr.to_string(),
                };
                to_header_value(&value)
            })
        },
        _ => handle_special_headers(buf, session, ctx),
    }
}

const HTTP_HEADER_PREFIX: &[u8] = b"$http_";
const HTTP_HEADER_PREFIX_LEN: usize = HTTP_HEADER_PREFIX.len();

#[inline]
fn handle_special_headers(
    buf: &[u8],
    session: &Session,
    ctx: &Ctx,
) -> Option<HeaderValue> {
    // Handle headers that reference other HTTP headers (e.g., $http_origin)
    if buf.starts_with(HTTP_HEADER_PREFIX) {
        return handle_http_header(buf, session);
    }
    // Handle environment variable references (e.g., $HOME)
    if buf.starts_with(b"$") {
        return handle_env_var(buf);
    }
    // Handle context value references (e.g., :connection_id)
    if buf.starts_with(b":") {
        return handle_context_value(buf, ctx);
    }
    None
}

#[inline]
fn handle_http_header(buf: &[u8], session: &Session) -> Option<HeaderValue> {
    // Skip the "$http_" prefix (6 bytes) and convert remaining bytes to header key
    let key = std::str::from_utf8(&buf[HTTP_HEADER_PREFIX_LEN..]).ok()?;
    // Look up and clone the header value from the session
    session.get_header(key).cloned()
}

#[inline]
fn handle_env_var(buf: &[u8]) -> Option<HeaderValue> {
    // Skip the "$" prefix and convert to environment variable name
    let var_name = std::str::from_utf8(&buf[1..]).ok()?;
    // Look up environment variable and convert to HeaderValue if found
    std::env::var(var_name)
        .ok()
        .and_then(|v| HeaderValue::from_str(&v).ok())
}

#[inline]
fn handle_context_value(buf: &[u8], ctx: &Ctx) -> Option<HeaderValue> {
    // Skip the ":" prefix and convert to context key
    let key = std::str::from_utf8(&buf[1..]).ok()?;
    // Pre-allocate buffer for value
    let mut value = BytesMut::with_capacity(20);
    // Append context value to buffer
    value = ctx.append_value(value, key);
    // Convert to HeaderValue if buffer is not empty
    if !value.is_empty() {
        HeaderValue::from_bytes(&value).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use pingora::proxy::Session;
    use pretty_assertions::assert_eq;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn test_convert_header_value() {
        let headers = ["Host: pingap.io"].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let default_state = Ctx {
            tls_version: Some("tls1.3".to_string()),
            remote_addr: Some("10.1.1.1".to_string()),
            remote_port: Some(6000),
            server_addr: Some("10.1.1.2".to_string()),
            server_port: Some(6001),
            upstream_address: "10.1.1.3:4123".to_string(),
            connection_id: 102,
            ..Default::default()
        };

        let value = convert_header_value(
            &HeaderValue::from_str("$host").unwrap(),
            &session,
            &Ctx {
                ..Default::default()
            },
        );
        assert_eq!(true, value.is_some());
        assert_eq!("pingap.io", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str("$scheme").unwrap(),
            &session,
            &Ctx {
                ..Default::default()
            },
        );
        assert_eq!(true, value.is_some());
        assert_eq!("http", value.unwrap().to_str().unwrap());
        let value = convert_header_value(
            &HeaderValue::from_str("$scheme").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("https", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str("$remote_addr").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("10.1.1.1", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str("$remote_port").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("6000", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str("$server_addr").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("10.1.1.2", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str("$server_port").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("6001", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str("$upstream_addr").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("10.1.1.3:4123", value.unwrap().to_str().unwrap());

        let value = convert_header_value(
            &HeaderValue::from_str(":connection_id").unwrap(),
            &session,
            &default_state,
        );
        assert_eq!(true, value.is_some());
        assert_eq!("102", value.unwrap().to_str().unwrap());

        let headers = ["X-Forwarded-For: 1.1.1.1, 2.2.2.2"].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("$proxy_add_x_forwarded_for").unwrap(),
            &session,
            &Ctx {
                remote_addr: Some("10.1.1.1".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(true, value.is_some());
        assert_eq!(
            "1.1.1.1, 2.2.2.2, 10.1.1.1",
            value.unwrap().to_str().unwrap()
        );

        let headers = [""].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("$proxy_add_x_forwarded_for").unwrap(),
            &session,
            &Ctx {
                remote_addr: Some("10.1.1.1".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(true, value.is_some());
        assert_eq!("10.1.1.1", value.unwrap().to_str().unwrap());

        let headers = [""].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("$upstream_addr").unwrap(),
            &session,
            &Ctx {
                upstream_address: "10.1.1.1:8001".to_string(),
                ..Default::default()
            },
        );
        assert_eq!(true, value.is_some());
        assert_eq!("10.1.1.1:8001", value.unwrap().to_str().unwrap());

        let headers = ["Origin: https://github.com"].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("$http_origin").unwrap(),
            &session,
            &Ctx::default(),
        );
        assert_eq!(true, value.is_some());
        assert_eq!("https://github.com", value.unwrap().to_str().unwrap());

        let headers = ["Origin: https://github.com"].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("$hostname").unwrap(),
            &session,
            &Ctx::default(),
        );
        assert_eq!(true, value.is_some());

        let headers = ["Origin: https://github.com"].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("$HOME").unwrap(),
            &session,
            &Ctx::default(),
        );
        assert_eq!(true, value.is_some());

        let headers = ["Origin: https://github.com"].join("\r\n");
        let input_header =
            format!("GET /vicanso/pingap?size=1 HTTP/1.1\r\n{headers}\r\n\r\n");
        let mock_io = Builder::new().read(input_header.as_bytes()).build();
        let mut session = Session::new_h1(Box::new(mock_io));
        session.read_request().await.unwrap();
        let value = convert_header_value(
            &HeaderValue::from_str("UUID").unwrap(),
            &session,
            &Ctx::default(),
        );
        assert_eq!(false, value.is_some());
    }
}
