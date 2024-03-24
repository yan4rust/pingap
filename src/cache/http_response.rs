use super::{HttpHeader, HTTP_HEADER_NO_STORE};
use bytes::Bytes;
use http::header;
use http::StatusCode;
use pingora::{http::ResponseHeader, proxy::Session};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default, Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub body: Bytes,
    pub max_age: Option<u32>,
    pub created_at: Option<u64>,
    pub private: Option<bool>,
    pub headers: Option<Vec<HttpHeader>>,
}

impl HttpResponse {
    pub fn get_response_header(&self) -> pingora::Result<ResponseHeader> {
        let fix_size = 3;
        let size = self
            .headers
            .as_ref()
            .map_or_else(|| fix_size, |headers| headers.len() + fix_size);
        let mut resp = ResponseHeader::build(self.status, Some(size))?;
        resp.insert_header(http::header::CONTENT_LENGTH, self.body.len().to_string())?;

        // set cache control
        if let Some(max_age) = self.max_age {
            let category = if self.private.unwrap_or_default() {
                "private"
            } else {
                "public"
            };
            if let Ok(value) =
                header::HeaderValue::from_str(&format!("{category}, max-age={max_age}"))
            {
                resp.insert_header(header::CACHE_CONTROL, value)?;
            }
        } else {
            let h = HTTP_HEADER_NO_STORE.clone();
            resp.insert_header(h.0, h.1)?;
        }

        if let Some(created_at) = self.created_at {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                - created_at;
            if let Ok(value) = header::HeaderValue::from_str(&secs.to_string()) {
                resp.insert_header(header::AGE, value)?;
            }
        }

        if let Some(headers) = &self.headers {
            for (name, value) in headers {
                resp.insert_header(name.to_owned(), value)?;
            }
        }
        Ok(resp)
    }
    pub async fn send(self, session: &mut Session) -> pingora::Result<usize> {
        let header = self.get_response_header()?;
        let size = self.body.len();
        session.write_response_header(Box::new(header)).await?;
        session.write_response_body(self.body).await?;
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::HttpResponse;
    use crate::cache::convert_headers;
    use bytes::Bytes;
    use http::StatusCode;
    use pretty_assertions::assert_eq;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn test_http_response() {
        let resp = HttpResponse {
            status: StatusCode::OK,
            body: Bytes::from("Hello world!"),
            max_age: Some(3600),
            created_at: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    - 10,
            ),
            private: Some(true),
            headers: Some(
                convert_headers(&[
                    "Contont-Type: application/json".to_string(),
                    "Content-Encoding: gzip".to_string(),
                ])
                .unwrap(),
            ),
        };

        let mut header = resp.get_response_header().unwrap();
        assert_eq!(true, !header.headers.get("Age").unwrap().is_empty());
        header.remove_header("Age").unwrap();

        assert_eq!(
            r###"ResponseHeader { base: Parts { status: 200, version: HTTP/1.1, headers: {"content-length": "12", "cache-control": "private, max-age=3600", "content-encoding": "gzip", "contont-type": "application/json"} }, header_name_map: Some({"content-length": CaseHeaderName(b"Content-Length"), "cache-control": CaseHeaderName(b"Cache-Control"), "content-encoding": CaseHeaderName(b"Content-Encoding"), "contont-type": CaseHeaderName(b"contont-type")}) }"###,
            format!("{header:?}")
        );
    }
}
