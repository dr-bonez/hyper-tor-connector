use std::sync::Arc;
use std::task::Poll;

pub use arti_client::Error;
use arti_client::{BootstrapBehavior, DataStream, TorClient};
use futures::future::BoxFuture;
use futures::FutureExt;
use hyper::client::connect::{Connected, Connection};
use hyper::Uri;
use tokio::io::{AsyncRead, AsyncWrite};
use tor_rtcompat::PreferredRuntime;
use tower::Service;

#[derive(Clone)]
pub struct TorConnector {
    tor_client: Arc<TorClient<PreferredRuntime>>,
}
impl TorConnector {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            tor_client: Arc::new(
                TorClient::builder()
                    .bootstrap_behavior(BootstrapBehavior::OnDemand)
                    .create_unbootstrapped()?,
            ),
        })
    }
}
impl std::fmt::Debug for TorConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TorConnector").finish()
    }
}

#[pin_project::pin_project]
pub struct TorStream(#[pin] pub DataStream);
impl Connection for TorStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}
impl AsyncWrite for TorStream {
    fn is_write_vectored(&self) -> bool {
        self.0.is_write_vectored()
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_flush(cx)
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().0.poll_shutdown(cx)
    }
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().0.poll_write(cx, buf)
    }
    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().0.poll_write_vectored(cx, bufs)
    }
}

impl AsyncRead for TorStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl Service<Uri> for TorConnector {
    type Response = TorStream;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let client = self.tor_client.clone();
        async move {
            Ok::<_, Error>(TorStream(
                client
                    .connect((
                        req.host().unwrap_or_default(),
                        req.port_u16().unwrap_or(match req.scheme_str() {
                            Some("https") | Some("wss") => 443,
                            _ => 80,
                        }),
                    ))
                    .await?,
            ))
        }
        .boxed()
    }
}
