use std::net::SocketAddr;
use std::task::Poll;

use futures::future::BoxFuture;
use futures::FutureExt;
use hyper::client::connect::{Connected, Connection};
use hyper::Uri;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
pub use tokio_socks::Error;
use tower::Service;

pub mod maybe;

#[derive(Debug, Clone)]
pub struct TorConnector {
    proxy_addr: SocketAddr,
}
impl TorConnector {
    pub fn new(proxy_addr: SocketAddr) -> Result<Self, Error> {
        Ok(Self { proxy_addr })
    }
}

#[pin_project::pin_project]
pub struct TorStream(#[pin] pub Socks5Stream<TcpStream>);
impl Connection for TorStream {
    fn connected(&self) -> Connected {
        self.0.connected()
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
        let proxy = self.proxy_addr;
        async move {
            Ok::<_, Error>(TorStream(
                Socks5Stream::connect(
                    proxy,
                    (
                        req.host().unwrap_or_default(),
                        req.port_u16().unwrap_or(match req.scheme_str() {
                            Some("https") | Some("wss") => 443,
                            _ => 80,
                        }),
                    ),
                )
                .await?,
            ))
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::client::Client;
    use hyper::Body;

    #[tokio::test]
    async fn get_torproject_page() {
        let client: Client<TorConnector, Body> =
            Client::builder().build(TorConnector::new(([127, 0, 0, 1], 9050).into()).unwrap());
        assert!(client
            .get(
                "http://2gzyxa5ihm7nsggfxnu52rck2vv4rvmdlkiu3zzui5du4xyclen53wid.onion"
                    .parse()
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
            .is_success());
    }
}
