use std::task::Poll;

use super::{TorConnector, TorStream};
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use hyper::client::connect::{Connected, Connection};
use hyper::client::HttpConnector;
use hyper::Uri;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tower::{BoxError, Service};

#[derive(Debug, Clone)]
pub enum MaybeTorConnector {
    ClearnetOnly(HttpConnector),
    Hybrid {
        clearnet: HttpConnector,
        tor: TorConnector,
    },
    TorOnly(TorConnector),
}

#[pin_project::pin_project(project = MaybeTorStreamProj)]
pub enum MaybeTorStream {
    Clearnet(#[pin] TcpStream),
    Tor(#[pin] TorStream),
}
impl Connection for MaybeTorStream {
    fn connected(&self) -> Connected {
        match self {
            MaybeTorStream::Clearnet(a) => a.connected(),
            MaybeTorStream::Tor(a) => a.connected(),
        }
    }
}
impl AsyncWrite for MaybeTorStream {
    fn is_write_vectored(&self) -> bool {
        match self {
            MaybeTorStream::Clearnet(a) => a.is_write_vectored(),
            MaybeTorStream::Tor(a) => a.is_write_vectored(),
        }
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.project() {
            MaybeTorStreamProj::Clearnet(a) => a.poll_flush(cx),
            MaybeTorStreamProj::Tor(a) => a.poll_flush(cx),
        }
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match self.project() {
            MaybeTorStreamProj::Clearnet(a) => a.poll_shutdown(cx),
            MaybeTorStreamProj::Tor(a) => a.poll_shutdown(cx),
        }
    }
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.project() {
            MaybeTorStreamProj::Clearnet(a) => a.poll_write(cx, buf),
            MaybeTorStreamProj::Tor(a) => a.poll_write(cx, buf),
        }
    }
    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        match self.project() {
            MaybeTorStreamProj::Clearnet(a) => a.poll_write_vectored(cx, bufs),
            MaybeTorStreamProj::Tor(a) => a.poll_write_vectored(cx, bufs),
        }
    }
}

impl AsyncRead for MaybeTorStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.project() {
            MaybeTorStreamProj::Clearnet(a) => a.poll_read(cx, buf),
            MaybeTorStreamProj::Tor(a) => a.poll_read(cx, buf),
        }
    }
}

impl Service<Uri> for MaybeTorConnector {
    type Response = MaybeTorStream;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        match self {
            MaybeTorConnector::ClearnetOnly(clearnet) => clearnet
                .call(req)
                .map_ok(MaybeTorStream::Clearnet)
                .map_err(BoxError::from)
                .boxed(),
            MaybeTorConnector::Hybrid { clearnet, tor } => {
                if req.host().unwrap_or_default().ends_with(".onion") {
                    tor.call(req)
                        .map_ok(MaybeTorStream::Tor)
                        .map_err(BoxError::from)
                        .boxed()
                } else {
                    clearnet
                        .call(req)
                        .map_ok(MaybeTorStream::Clearnet)
                        .map_err(BoxError::from)
                        .boxed()
                }
            }
            MaybeTorConnector::TorOnly(tor) => tor
                .call(req)
                .map_ok(MaybeTorStream::Tor)
                .map_err(BoxError::from)
                .boxed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::client::Client;
    use hyper::Body;

    #[tokio::test]
    async fn get_torproject_page() {
        #[cfg(feature = "socks")]
        let tor = TorConnector::new(([127, 0, 0, 1], 9050).into()).unwrap();
        #[cfg(feature = "arti")]
        let tor = TorConnector::new().unwrap();
        let client: Client<MaybeTorConnector, Body> =
            Client::builder().build(MaybeTorConnector::Hybrid {
                tor,
                clearnet: HttpConnector::new(),
            });
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
        client
            .get("http://torproject.org".parse().unwrap())
            .await
            .unwrap();
    }
}
