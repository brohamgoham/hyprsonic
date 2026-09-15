use capital_core::{Clock, EvidenceStore};
use reqwest::{blocking::Client, redirect::Policy};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

pub struct LocalJournal {
    pub path: PathBuf,
    next: AtomicU64,
}
impl LocalJournal {
    pub fn create(base: &Path, now: u64) -> Result<Self, String> {
        // Private files, exclusive run directories; never append into an old capture.
        for part in base.ancestors().collect::<Vec<_>>().into_iter().rev() {
            if part.as_os_str().is_empty() {
                continue;
            }
            if let Ok(meta) = fs::symlink_metadata(part) {
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err("journal path must contain only directories, not symlinks".into());
                }
            } else {
                private_dir(part)?;
            }
        }
        let path = base.join(format!("{now}-{}", std::process::id()));
        private_dir(&path)?;
        Ok(Self {
            path,
            next: AtomicU64::new(1),
        })
    }
    pub fn report(&self, value: &Value) -> Result<(), String> {
        private_file(
            &self.path.join("report.json"),
            &serde_json::to_vec_pretty(value).map_err(|_| "report serialization failed")?,
        )
    }
}
fn private_dir(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(path)
        .map_err(|_| "cannot create private journal directory".into())
}
fn private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "cannot create evidence file")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "cannot persist evidence file".into())
}
impl EvidenceStore for LocalJournal {
    fn append(&self, record: &[u8]) -> Result<String, String> {
        let id = format!("{:06}.json", self.next.fetch_add(1, Ordering::Relaxed));
        private_file(&self.path.join(&id), record)?;
        Ok(id)
    }
}

pub struct Request {
    pub url: String,
    pub body: Option<Value>,
    pub query: Vec<(String, String)>,
}
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}
pub trait Transport: Sync {
    fn send(&self, request: &Request) -> Result<Reply, String>;
}
pub struct Http {
    client: Client,
}
impl Http {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            client: Client::builder()
                .https_only(true)
                .redirect(Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(12))
                .user_agent("hyprsonic/0.1 observation")
                .build()
                .map_err(|_| "cannot initialize HTTPS client")?,
        })
    }
}
impl Transport for Http {
    fn send(&self, request: &Request) -> Result<Reply, String> {
        for attempt in 0..3 {
            let r = if let Some(body) = &request.body {
                self.client.post(&request.url).json(body)
            } else {
                self.client.get(&request.url)
            };
            let response = r.query(&request.query).send().map_err(|e| {
                if e.is_timeout() {
                    "TIMEOUT"
                } else {
                    "TRANSPORT_ERROR"
                }
            })?;
            let status = response.status().as_u16();
            if matches!(status, 429 | 503) && attempt < 2 {
                let delay = response
                    .headers()
                    .get("retry-after")
                    .map(|h| {
                        h.to_str()
                            .ok()
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(u64::MAX)
                    })
                    .unwrap_or(1 << attempt);
                if delay > 3 {
                    return Ok(Reply {
                        status,
                        body: vec![],
                    });
                }
                std::thread::sleep(Duration::from_secs(delay));
                continue;
            }
            if !(200..300).contains(&status) {
                return Ok(Reply {
                    status,
                    body: vec![],
                });
            }
            let mut body = Vec::new();
            response
                .take(2_097_153)
                .read_to_end(&mut body)
                .map_err(|_| "BODY_READ_ERROR")?;
            if body.len() > 2_097_152 {
                return Err("RESPONSE_TOO_LARGE".into());
            }
            return Ok(Reply { status, body });
        }
        unreachable!()
    }
}

pub struct Context<'a> {
    pub transport: &'a dyn Transport,
    pub store: &'a dyn EvidenceStore,
    pub clock: &'a dyn Clock,
}
impl Context<'_> {
    pub fn read(
        &self,
        alias: &str,
        endpoint: &str,
        request: Request,
    ) -> Result<(Value, capital_core::EvidenceRef), String> {
        let started = self.clock.now_ms();
        let reply = self.transport.send(&request);
        let received = self.clock.now_ms();
        let (status, parsed, error) = match reply {
            Err(code) => (None, None, Some(code)),
            Ok(r) if !(200..300).contains(&r.status) => {
                (Some(r.status), None, Some(format!("HTTP_{}", r.status)))
            }
            Ok(r) => match serde_json::from_slice::<Value>(&r.body) {
                // RPC error messages may echo sensitive provider URLs. Retain a code, not their text.
                Ok(v)
                    if request
                        .body
                        .as_ref()
                        .is_some_and(|b| b.get("jsonrpc").is_some())
                        && v.get("error").is_some() =>
                {
                    (Some(r.status), None, Some("RPC_ERROR".into()))
                }
                Ok(v) => (Some(r.status), Some(v), None),
                Err(_) => (Some(r.status), None, Some("INVALID_JSON".into())),
            },
        };
        let source_time = parsed
            .as_ref()
            .and_then(|v| v.get("time"))
            .and_then(Value::as_u64);
        // Never serialize the transport URL: RPC provider credentials may live in its path/query.
        let id=self.store.append(&serde_json::to_vec_pretty(&json!({"schema_version":1,"adapter_version":"phase1-v1","alias":alias,
            "endpoint":endpoint,"request_body":request.body,"query":request.query,"started_at_ms":started,
            "received_at_ms":received,"source_time_ms":source_time,"http_status":status,"error":error,"response":parsed
        })).map_err(|_|"evidence serialization failed")?).map_err(|_|"JOURNAL_WRITE_FAILED")?;
        if let Some(error) = error {
            return Err(format!("{error} (evidence {id})"));
        }
        Ok((
            parsed.ok_or("missing response")?,
            capital_core::EvidenceRef {
                id,
                endpoint: endpoint.into(),
                received_at_ms: received,
                source_time_ms: source_time,
                block: None,
            },
        ))
    }
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use std::net::TcpListener;
    fn serve(replies: Vec<String>) -> (String, std::thread::JoinHandle<()>) {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", socket.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            for response in replies {
                let (mut stream, _) = socket.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = [0; 4096];
                let _ = stream.read(&mut bytes).unwrap();
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (url, handle)
    }
    fn local_http() -> Http {
        Http {
            client: Client::builder()
                .no_proxy()
                .redirect(Policy::none())
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap(),
        }
    }
    #[test]
    fn rate_limit_retries_are_bounded_and_long_retry_after_returns_promptly() {
        let (url,server)=serve(vec!["HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".into()]);
        let r = local_http()
            .send(&Request {
                url,
                body: None,
                query: vec![],
            })
            .unwrap();
        assert_eq!(r.status, 200);
        server.join().unwrap();
        let (url,server)=serve(vec!["HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()]);
        assert_eq!(
            local_http()
                .send(&Request {
                    url,
                    body: None,
                    query: vec![]
                })
                .unwrap()
                .status,
            429
        );
        server.join().unwrap();
    }
    #[test]
    fn redirects_are_not_followed_and_oversized_bodies_are_rejected() {
        let (url,server)=serve(vec!["HTTP/1.1 302 Found\r\nLocation: https://must-not-contact.invalid\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()]);
        assert_eq!(
            local_http()
                .send(&Request {
                    url,
                    body: None,
                    query: vec![]
                })
                .unwrap()
                .status,
            302
        );
        server.join().unwrap();
        let body = "x".repeat(2_097_153);
        let (url, server) = serve(vec![format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )]);
        assert!(
            matches!(local_http().send(&Request{url,body:None,query:vec![]}),Err(e) if e=="RESPONSE_TOO_LARGE")
        );
        server.join().unwrap();
    }
    #[test]
    fn production_client_rejects_plain_http() {
        assert!(
            Http::new()
                .unwrap()
                .send(&Request {
                    url: "http://127.0.0.1:1".into(),
                    body: None,
                    query: vec![]
                })
                .is_err()
        );
    }
}
