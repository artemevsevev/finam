use std::sync::{Arc, Mutex};

use thiserror::Error;
use tonic::{
    metadata::errors::InvalidMetadataValue,
    service::{Interceptor, interceptor::InterceptedService},
    transport::{Channel, ClientTlsConfig},
};

use crate::proto::grpc::tradeapi::v1::{
    accounts::accounts_service_client::AccountsServiceClient,
    assets::assets_service_client::AssetsServiceClient,
    auth::{AuthRequest, auth_service_client::AuthServiceClient},
    marketdata::market_data_service_client::MarketDataServiceClient,
    orders::orders_service_client::OrdersServiceClient,
};

pub mod proto;

#[derive(Clone)]
pub struct FinamSdk {
    accounts: AccountsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    assets: AssetsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    auth: AuthServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    market_data: MarketDataServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    orders: OrdersServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
}

impl FinamSdk {
    pub async fn new(secret: &str) -> Result<Self, FinamSdkError> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let channel = Channel::from_static("https://api.finam.ru")
            .tls_config(tls)?
            .connect()
            .await?;

        let interceptor = FinamSdkInterceptor::new(secret, channel.clone()).await?;

        Ok(Self {
            accounts: AccountsServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            assets: AssetsServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            auth: AuthServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            market_data: MarketDataServiceClient::with_interceptor(
                channel.clone(),
                interceptor.clone(),
            ),
            orders: OrdersServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
        })
    }

    pub fn accounts(
        &self,
    ) -> AccountsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.accounts.clone()
    }

    pub fn assets(&self) -> AssetsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.assets.clone()
    }

    pub fn auth(&self) -> AuthServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.auth.clone()
    }

    pub fn market_data(
        &self,
    ) -> MarketDataServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.market_data.clone()
    }

    pub fn orders(&self) -> OrdersServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.orders.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FinamSdkInterceptor {
    jwt_token: Arc<Mutex<String>>,
}

impl FinamSdkInterceptor {
    pub async fn new(secret: &str, channel: Channel) -> Result<Self, FinamSdkError> {
        let token = Arc::new(Mutex::new(
            generate_jwt_token(channel.clone(), secret.to_string()).await?,
        ));

        let secret = secret.to_string();
        let updating_token = token.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;

                let jwt_token = match generate_jwt_token(channel.clone(), secret.clone()).await {
                    Ok(value) => value,
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        log::error!("Failed to generate JWT token. Waiting for 5 seconds...");
                        continue;
                    }
                };

                let token = updating_token.clone();
                *token.lock().unwrap() = jwt_token;
            }
        });

        Ok(Self {
            jwt_token: token.clone(),
        })
    }

    pub fn get_jwt_token(&self) -> Result<String, tonic::Status> {
        Ok(self
            .jwt_token
            .lock()
            .map_err(|_| tonic::Status::internal("Can't lock JWT token mutex"))?
            .clone())
    }
}

impl Interceptor for FinamSdkInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        let jwt_token = self
            .get_jwt_token()?
            .parse()
            .map_err(|_| tonic::Status::internal("Invalid JWT token"))?;

        request.metadata_mut().append("authorization", jwt_token);

        Ok(request)
    }
}

async fn generate_jwt_token(channel: Channel, secret: String) -> Result<String, FinamSdkError> {
    let mut auth_service_client = AuthServiceClient::new(channel);
    let response = auth_service_client
        .auth(AuthRequest { secret })
        .await?
        .into_inner();

    Ok(response.token)
}

#[derive(Error, Debug)]
pub enum FinamSdkError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    #[error(transparent)]
    Status(#[from] tonic::Status),
    #[error(transparent)]
    InvalidMetadataValue(#[from] InvalidMetadataValue),
}
