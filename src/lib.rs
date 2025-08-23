use thiserror::Error;
use tonic::transport::{Channel, ClientTlsConfig};

#[derive(Clone)]
pub struct FinamSdk {
    channel: Channel,
}

#[derive(Error, Debug)]
pub enum FinamSdkError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    #[error(transparent)]
    Status(#[from] tonic::Status),
}

impl FinamSdk {
    pub async fn new(token: &str) -> Result<Self, FinamSdkError> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let channel = Channel::from_static("https://api.finam.ru")
            .tls_config(tls)?
            .connect()
            .await?;

        Ok(Self { channel })
    }
}
