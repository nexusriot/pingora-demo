use async_trait::async_trait;
use http::{Response, StatusCode, Version};
// use log::info; // optional

use pingora::{
    apps::http_app::ServeHttp,
    protocols::http::ServerSession,
    server::Server,
    services::listening::Service,
    Result,
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Hello {
    state: Arc<State>,
}

struct State {
    started_at: Instant,
    requests: AtomicU64,
    pid: u32,
}

impl Hello {
    fn new() -> Self {
        Self {
            state: Arc::new(State {
                started_at: Instant::now(),
                requests: AtomicU64::new(0),
                pid: std::process::id(),
            }),
        }
    }
}

#[derive(Serialize)]
struct HelloResp<'a> {
    response: &'a str,
}

#[derive(Serialize)]
struct HealthResp<'a> {
    health: &'a str,
}

#[derive(Serialize)]
struct StatusResp {
    uptime_ms: u128,
    uptime_human: String,
    requests: u64,
    pid: u32,
    service: &'static str,
    version: &'static str,
}

fn json(body: &str, status: StatusCode) -> Response<Vec<u8>> {
    let bytes = body.as_bytes().to_vec();
    Response::builder()
        .version(Version::HTTP_11)
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("content-length", bytes.len().to_string())
        .header("connection", "close")
        .body(bytes)
        .unwrap()
}

#[async_trait]
impl ServeHttp for Hello {
    async fn response(&self, sess: &mut ServerSession) -> Response<Vec<u8>> {
        // NOTE: `uri` is a field on RequestHeader in pingora 0.6
        let path = sess.req_header().uri.path();

        self.state.requests.fetch_add(1, Ordering::Relaxed);

        match path {
            "/hello" => {
                let payload = serde_json::to_string(&HelloResp { response: "hello" }).unwrap();
                json(&payload, StatusCode::OK)
            }
            "/health" => {
                let payload = serde_json::to_string(&HealthResp { health: "ok" }).unwrap();
                json(&payload, StatusCode::OK)
            }
            "/status" => {
                let up = self.state.started_at.elapsed();
                let payload = serde_json::to_string(&StatusResp {
                    uptime_ms: up.as_millis(),
                    uptime_human: human(up),
                    requests: self.state.requests.load(Ordering::Relaxed),
                    pid: self.state.pid,
                    service: "pingora-hello-router",
                    version: env!("CARGO_PKG_VERSION"),
                })
                .unwrap();
                json(&payload, StatusCode::OK)
            }
            _ => {
                let payload =
                    serde_json::json!({ "error": "not_found", "path": path }).to_string();
                json(&payload, StatusCode::NOT_FOUND)
            }
        }
    }
}

fn human(d: Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn main() -> Result<()> {
    env_logger::init();

    let mut server = Server::new(None)?;
    server.bootstrap();

    let app = Hello::new();
    let mut svc = Service::new("hello-router".to_string(), app);
    svc.add_tcp("0.0.0.0:8080");

    server.add_service(svc);
    server.run_forever();
}
