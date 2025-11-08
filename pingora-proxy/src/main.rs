use async_trait::async_trait;
use pingora_core::{server::Server, upstreams::peer::HttpPeer, Result};
use pingora_proxy::{http_proxy_service, ProxyHttp, Session};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use log::info;

struct SimpleProxy {
    req_id: Arc<AtomicU64>,
}

impl SimpleProxy {
    fn new() -> Self {
        Self { req_id: Arc::new(AtomicU64::new(1)) }
    }
}

#[async_trait]
impl ProxyHttp for SimpleProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX { () }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // Request metadata (method & path)
        let hdr = session.req_header();

        let method = hdr.method.to_string();
        let path   = hdr.uri.path().to_string();
        // TODO: get from env
        let upstream_host = "127.0.0.1";
        let upstream_port = 9020;

        // Increment a simple request id for logging correlation
        let id = self.req_id.fetch_add(1, Ordering::Relaxed);

        info!("[req #{id}] {} {} -> upstream {}:{}",
              method, path, upstream_host, upstream_port);

        Ok(Box::new(HttpPeer::new((upstream_host, upstream_port), false, String::new())))
    }
}

fn main() -> Result<()> {
    // Enable logs: RUST_LOG=info cargo run
    env_logger::init();

    let mut server = Server::new(None)?;
    server.bootstrap();

    let mut svc = http_proxy_service(&server.configuration, SimpleProxy::new());
    svc.add_tcp("0.0.0.0:8081");

    server.add_service(svc);
    server.run_forever();
}
