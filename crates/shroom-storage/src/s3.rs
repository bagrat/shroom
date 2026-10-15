//! A minimal S3 client over the signer: PUT, HEAD, GET, DELETE and ListObjectsV2,
//! path-style. Every call is idempotent by key, so a retry is always safe.

use std::time::Duration;

use crate::sigv4::{Credentials, amz_date_now, sign_with};

#[derive(Clone, Debug)]
pub struct Bucket {
    /// e.g. `https://<account>.r2.cloudflarestorage.com`
    pub endpoint: String,
    /// `auto` for R2.
    pub region: String,
    pub name: String,
    pub credentials: Credentials,
}

#[derive(Debug)]
pub enum S3Error {
    /// The store answered with a non-success status.
    Status { status: u16, body: String },
    Transport(String),
}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3Error::Status { status, body } => write!(f, "HTTP {status}: {}", body.chars().take(300).collect::<String>()),
            S3Error::Transport(e) => write!(f, "transport: {e}"),
        }
    }
}

impl std::error::Error for S3Error {}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub key: String,
    pub size: u64,
    /// As the listing gives it (quoted), so a reader re-fetches only what changed.
    pub etag: Option<String>,
}

pub struct Client {
    agent: ureq::Agent,
    bucket: Bucket,
}

impl Client {
    pub fn new(bucket: Bucket) -> Client {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
            .build();
        Client { agent: config.into(), bucket }
    }

    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }

    fn url(&self, key: &str) -> String {
        let path: Vec<String> = key.split('/').map(crate::sigv4::encode).collect();
        format!("{}/{}/{}", self.bucket.endpoint.trim_end_matches('/'), self.bucket.name, path.join("/"))
    }

    fn send(&self, method: &str, url: &str, body: &[u8], content_type: Option<&str>) -> Result<(u16, Vec<u8>), S3Error> {
        self.send_with(method, url, body, content_type, &[]).map(|(s, b, _)| (s, b))
    }

    fn send_with(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        content_type: Option<&str>,
        extra: &[(&str, &str)],
    ) -> Result<(u16, Vec<u8>, Option<String>), S3Error> {
        self.send_signed(method, url, body, content_type, extra, &[])
    }

    /// One request; `signed` headers (`x-amz-*`) are signed as well as sent, `extra` only sent.
    fn send_signed(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        content_type: Option<&str>,
        extra: &[(&str, &str)],
        signed: &[(&str, &str)],
    ) -> Result<(u16, Vec<u8>, Option<String>), S3Error> {
        let signature = sign_with(method, url, body, &self.bucket.region, &self.bucket.credentials, &amz_date_now(), signed);
        let mut req = ureq::http::Request::builder().method(method).uri(url);
        for (k, v) in signature.headers() {
            req = req.header(k, v);
        }
        if let Some(ct) = content_type {
            req = req.header("content-type", ct);
        }
        for (k, v) in extra.iter().chain(signed) {
            req = req.header(*k, *v);
        }
        let req = req.body(body.to_vec()).map_err(|e| S3Error::Transport(e.to_string()))?;
        let mut res = self.agent.run(req).map_err(|e| S3Error::Transport(e.to_string()))?;
        let status = res.status().as_u16();
        let etag = res.headers().get("etag").and_then(|v| v.to_str().ok()).map(String::from);
        let bytes = if method == "HEAD" {
            Vec::new()
        } else {
            res.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().map_err(|e| S3Error::Transport(e.to_string()))?
        };
        Ok((status, bytes, etag))
    }

    fn expect_ok(status: u16, body: Vec<u8>) -> Result<Vec<u8>, S3Error> {
        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(S3Error::Status { status, body: String::from_utf8_lossy(&body).into_owned() })
        }
    }

    pub fn put(&self, key: &str, body: &[u8], content_type: &str) -> Result<(), S3Error> {
        let (status, b) = self.send("PUT", &self.url(key), body, Some(content_type))?;
        Self::expect_ok(status, b).map(|_| ())
    }

    /// Creates the object only if no object has that key (`If-None-Match: *`).
    /// Returns `false` when it already existed (412) — the create-only claim R10.2 uses
    /// for `meta/<id>.json`, so an id collision can never overwrite.
    pub fn put_if_absent(&self, key: &str, body: &[u8], content_type: &str) -> Result<bool, S3Error> {
        match self.send_with("PUT", &self.url(key), body, Some(content_type), &[("if-none-match", "*")])? {
            (200..=299, _, _) => Ok(true),
            (412, _, _) => Ok(false),
            (status, b, _) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&b).into_owned() }),
        }
    }

    /// [`put_if_absent`](Self::put_if_absent) with user metadata: each `(name, value)` rides
    /// as a signed `x-amz-meta-<name>` header (R2's `customMetadata`) — the ownership
    /// ledger's `m` (WORKER-INTERFACE §2.4), which the Worker's listings read without a GET.
    /// Names are lower-case ASCII; values printable ASCII.
    pub fn put_if_absent_with_meta(&self, key: &str, body: &[u8], content_type: &str, meta: &[(&str, &str)]) -> Result<bool, S3Error> {
        let names: Vec<String> = meta.iter().map(|(k, _)| format!("x-amz-meta-{}", k.to_ascii_lowercase())).collect();
        let signed: Vec<(&str, &str)> = names.iter().zip(meta).map(|(n, (_, v))| (n.as_str(), *v)).collect();
        match self.send_signed("PUT", &self.url(key), body, Some(content_type), &[("if-none-match", "*")], &signed)? {
            (200..=299, _, _) => Ok(true),
            (412, _, _) => Ok(false),
            (status, b, _) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&b).into_owned() }),
        }
    }

    /// Replaces the object only if it still has `etag` (`If-Match`) — optimistic
    /// concurrency for read-modify-write. Returns `false` on 412 (someone wrote first).
    pub fn put_if_match(&self, key: &str, body: &[u8], content_type: &str, etag: &str) -> Result<bool, S3Error> {
        match self.send_with("PUT", &self.url(key), body, Some(content_type), &[("if-match", etag)])? {
            (200..=299, _, _) => Ok(true),
            (412, _, _) => Ok(false),
            (status, b, _) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&b).into_owned() }),
        }
    }

    /// The object's bytes and ETag, or `None` if it doesn't exist.
    pub fn get_with_etag(&self, key: &str) -> Result<Option<(Vec<u8>, String)>, S3Error> {
        match self.send_with("GET", &self.url(key), b"", None, &[])? {
            (200, body, Some(etag)) => Ok(Some((body, etag))),
            (200, _, None) => Err(S3Error::Status { status: 200, body: "no ETag".into() }),
            (404, _, _) => Ok(None),
            (status, body, _) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&body).into_owned() }),
        }
    }

    /// `true` if the object exists.
    pub fn head(&self, key: &str) -> Result<bool, S3Error> {
        match self.send("HEAD", &self.url(key), b"", None)? {
            (200, _) => Ok(true),
            (404, _) => Ok(false),
            (status, body) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&body).into_owned() }),
        }
    }

    /// The object's bytes, or `None` if it doesn't exist.
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, S3Error> {
        match self.send("GET", &self.url(key), b"", None)? {
            (200, body) => Ok(Some(body)),
            (404, _) => Ok(None),
            (status, body) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&body).into_owned() }),
        }
    }

    /// Deleting a missing object succeeds (idempotent).
    pub fn delete(&self, key: &str) -> Result<(), S3Error> {
        match self.send("DELETE", &self.url(key), b"", None)? {
            (200..=299, _) | (404, _) => Ok(()),
            (status, body) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&body).into_owned() }),
        }
    }

    /// Starts a multipart upload; returns its upload id.
    pub fn create_multipart(&self, key: &str, content_type: &str) -> Result<String, S3Error> {
        let (status, body) = self.send("POST", &format!("{}?uploads=", self.url(key)), b"", Some(content_type))?;
        let xml = String::from_utf8_lossy(&Self::expect_ok(status, body)?).into_owned();
        between(&xml, "<UploadId>", "</UploadId>").map(unescape).ok_or(S3Error::Status { status, body: xml })
    }

    /// Uploads part `number` (1-based); returns its ETag for the completion.
    pub fn upload_part(&self, key: &str, upload_id: &str, number: u32, bytes: &[u8]) -> Result<String, S3Error> {
        let url = format!("{}?partNumber={number}&uploadId={}", self.url(key), crate::sigv4::encode(upload_id));
        let (status, body, etag) = self.send_with("PUT", &url, bytes, None, &[])?;
        Self::expect_ok(status, body)?;
        etag.ok_or(S3Error::Status { status, body: "no ETag on the part".into() })
    }

    /// Completes a multipart upload from its parts' ETags, in order.
    pub fn complete_multipart(&self, key: &str, upload_id: &str, etags: &[String]) -> Result<(), S3Error> {
        let mut xml = String::from("<CompleteMultipartUpload>");
        for (i, etag) in etags.iter().enumerate() {
            xml.push_str(&format!("<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>", i + 1, xml_escape(etag)));
        }
        xml.push_str("</CompleteMultipartUpload>");
        let url = format!("{}?uploadId={}", self.url(key), crate::sigv4::encode(upload_id));
        let (status, body) = self.send("POST", &url, xml.as_bytes(), Some("application/xml"))?;
        let body = Self::expect_ok(status, body)?;
        // S3 can report a failed completion inside a 200.
        let text = String::from_utf8_lossy(&body);
        if text.contains("<Error>") {
            return Err(S3Error::Status { status, body: text.into_owned() });
        }
        Ok(())
    }

    pub fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), S3Error> {
        let url = format!("{}?uploadId={}", self.url(key), crate::sigv4::encode(upload_id));
        match self.send("DELETE", &url, b"", None)? {
            (200..=299, _) | (404, _) => Ok(()),
            (status, body) => Err(S3Error::Status { status, body: String::from_utf8_lossy(&body).into_owned() }),
        }
    }

    /// Uploads a file of any size: one PUT up to `single_put_max`, multipart above it in
    /// `part_size` pieces (all but the last equal), reading one part at a time.
    pub fn put_file(&self, key: &str, path: &std::path::Path, content_type: &str, single_put_max: u64, part_size: u64) -> Result<(), S3Error> {
        use std::io::Read;
        let io = |e: std::io::Error| S3Error::Transport(e.to_string());
        let len = std::fs::metadata(path).map_err(io)?.len();
        if len <= single_put_max {
            let bytes = std::fs::read(path).map_err(io)?;
            return self.put(key, &bytes, content_type);
        }
        let upload_id = self.create_multipart(key, content_type)?;
        let result = (|| {
            let mut file = std::fs::File::open(path).map_err(io)?;
            let mut etags = Vec::new();
            let mut buf = vec![0u8; part_size as usize];
            let mut number = 1;
            loop {
                let mut filled = 0;
                while filled < buf.len() {
                    let n = file.read(&mut buf[filled..]).map_err(io)?;
                    if n == 0 {
                        break;
                    }
                    filled += n;
                }
                if filled == 0 {
                    break;
                }
                etags.push(self.upload_part(key, &upload_id, number, &buf[..filled])?);
                number += 1;
                if filled < buf.len() {
                    break;
                }
            }
            self.complete_multipart(key, &upload_id, &etags)
        })();
        if result.is_err() {
            let _ = self.abort_multipart(key, &upload_id);
        }
        result
    }

    /// Every object under `prefix`, following continuation tokens.
    pub fn list(&self, prefix: &str) -> Result<Vec<Object>, S3Error> {
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut url = format!(
                "{}/{}?list-type=2&prefix={}",
                self.bucket.endpoint.trim_end_matches('/'),
                self.bucket.name,
                crate::sigv4::encode(prefix)
            );
            if let Some(t) = &token {
                url.push_str(&format!("&continuation-token={}", crate::sigv4::encode(t)));
            }
            let (status, body) = self.send("GET", &url, b"", None)?;
            let xml = String::from_utf8_lossy(&Self::expect_ok(status, body)?).into_owned();
            let page = parse_list(&xml);
            out.extend(page.objects);
            match page.next {
                Some(t) if page.truncated => token = Some(t),
                _ => break,
            }
        }
        Ok(out)
    }
}

pub(crate) struct ListPage {
    pub objects: Vec<Object>,
    pub truncated: bool,
    pub next: Option<String>,
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = s.find(open)? + open.len();
    let end = s[start..].find(close)? + start;
    Some(&s[start..end])
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#39;", "'").replace("&amp;", "&")
}

/// ListObjectsV2's XML, read with plain string scanning (keys are ids and file names).
pub(crate) fn parse_list(xml: &str) -> ListPage {
    let mut objects = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<Contents>") {
        let after = &rest[i + "<Contents>".len()..];
        let Some(end) = after.find("</Contents>") else { break };
        let block = &after[..end];
        if let Some(key) = between(block, "<Key>", "</Key>") {
            let size = between(block, "<Size>", "</Size>").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
            let etag = between(block, "<ETag>", "</ETag>").map(|e| unescape(e).trim().to_string());
            objects.push(Object { key: unescape(key), size, etag });
        }
        rest = &after[end..];
    }
    ListPage {
        objects,
        truncated: between(xml, "<IsTruncated>", "</IsTruncated>").map(|v| v.trim() == "true").unwrap_or(false),
        next: between(xml, "<NextContinuationToken>", "</NextContinuationToken>").map(unescape),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_list_page() {
        let xml = "<ListBucketResult><IsTruncated>true</IsTruncated><Contents><Key>v/a&amp;b/seg_00000.m4s</Key><Size>12</Size></Contents>\
                   <Contents><Key>v/x/init.mp4</Key><Size>804</Size></Contents><NextContinuationToken>tok&amp;1</NextContinuationToken></ListBucketResult>";
        let p = parse_list(xml);
        assert_eq!(p.objects, vec![
            Object { key: "v/a&b/seg_00000.m4s".into(), size: 12, etag: None },
            Object { key: "v/x/init.mp4".into(), size: 804, etag: None },
        ]);
        assert!(p.truncated);
        assert_eq!(p.next.as_deref(), Some("tok&1"));
    }

    /// One request, as a local server sees it: the request line and headers (lower-case
    /// names), answered with `status`.
    fn capture(status: u16) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut lines = Vec::new();
            let mut length = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim_end().to_string();
                if line.is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap_or(0);
                }
                lines.push(line);
            }
            let mut body = vec![0u8; length];
            std::io::Read::read_exact(&mut reader, &mut body).unwrap();
            let mut s = stream;
            write!(s, "HTTP/1.1 {status} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").unwrap();
            lines
        });
        (addr, handle)
    }

    fn client(endpoint: &str) -> Client {
        Client::new(Bucket {
            endpoint: endpoint.into(),
            region: "auto".into(),
            name: "shroom-space".into(),
            credentials: Credentials { access_key_id: "id".into(), secret_access_key: "secret".into(), session_token: None },
        })
    }

    #[test]
    fn a_ledger_entry_carries_its_metadata_signed_and_create_only() {
        let (addr, seen) = capture(200);
        let made = client(&addr).put_if_absent_with_meta(".shroom/registry/recs/AbCdEfGhIjKlMnOp", b"{}", "application/json", &[("m", "m1")]).unwrap();
        assert!(made);
        let lines = seen.join().unwrap();
        let header = |name: &str| lines.iter().find_map(|l| l.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_string()));
        assert_eq!(lines[0], "PUT /shroom-space/.shroom/registry/recs/AbCdEfGhIjKlMnOp HTTP/1.1");
        assert_eq!(header("x-amz-meta-m").as_deref(), Some("m1"));
        assert_eq!(header("if-none-match").as_deref(), Some("*"));
        let auth = header("authorization").unwrap();
        assert!(auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-meta-m,"), "{auth}");
        // Already there: 412 is “not made”, never an error (idempotent).
        let (addr, _) = capture(412);
        assert!(!client(&addr).put_if_absent_with_meta("k", b"{}", "application/json", &[("m", "m1")]).unwrap());
    }

    #[test]
    fn an_empty_list_is_empty() {
        let p = parse_list("<ListBucketResult><IsTruncated>false</IsTruncated></ListBucketResult>");
        assert!(p.objects.is_empty() && !p.truncated && p.next.is_none());
    }
}
