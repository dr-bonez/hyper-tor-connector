pub mod maybe;

#[cfg(feature = "socks")]
mod socks;
#[cfg(feature = "socks")]
pub use socks::*;

#[cfg(feature = "arti")]
mod arti;
#[cfg(feature = "arti")]
pub use arti::*;

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
        let client: Client<TorConnector, Body> = Client::builder().build(tor);
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
