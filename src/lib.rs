use thiserror::Error;
use tonic::transport::{Channel, ClientTlsConfig};

pub mod google {
    pub mod api {
        include!("google.api.rs");
    }
    pub mod r#type {
        include!("google.r#type.rs");
    }
}

pub mod grpc {
    pub mod tradeapi {
        pub mod v1 {
            include!("grpc.tradeapi.v1.rs");

            pub mod accounts {
                include!("grpc.tradeapi.v1.accounts.rs");
            }

            pub mod assets {
                include!("grpc.tradeapi.v1.assets.rs");
            }

            pub mod auth {
                include!("grpc.tradeapi.v1.auth.rs");
            }

            pub mod marketdata {
                include!("grpc.tradeapi.v1.marketdata.rs");
            }

            pub mod orders {
                include!("grpc.tradeapi.v1.orders.rs");
            }
        }
    }
}

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
