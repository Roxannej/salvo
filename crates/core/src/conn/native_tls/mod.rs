//! native_tls module
pub mod listener;
pub use listener::NativeTlsListener;

mod config;
pub use config::NativeTlsConfig;

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    use super::*;
    use crate::conn::{TcpListener, Accepted,Acceptor, Listener};

    #[tokio::test]
    async fn test_native_tls_listener() {
        let mut acceptor = TcpListener::new("127.0.0.1:0")
            .native_tls(
                NativeTlsConfig::new()
                    .with_pkcs12(include_bytes!("../../../certs/identity.p12").as_ref())
                    .with_password("mypass"),
            )
            .bind()
            .await;
        let addr = acceptor.holdings()[0].local_addr.clone().into_std().unwrap();

        tokio::spawn(async move {
            let connector = tokio_native_tls::TlsConnector::from(
                tokio_native_tls::native_tls::TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .unwrap(),
            );
            let stream = TcpStream::connect(addr).await.unwrap();
            let mut stream = connector.connect("127.0.0.1", stream).await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let Accepted { mut conn, .. } = acceptor.accept().await.unwrap();
        assert_eq!(conn.read_i32().await.unwrap(), 10);
    }
}
