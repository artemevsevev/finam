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

/// Основной клиент SDK для работы с API Финам.
///
/// Предоставляет доступ к различным сервисам API Финам через gRPC.
/// Включает в себя клиенты для работы со счетами, активами, аутентификацией,
/// рыночными данными и ордерами.
#[derive(Clone)]
pub struct FinamSdk {
    accounts: AccountsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    assets: AssetsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    auth: AuthServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    market_data: MarketDataServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
    orders: OrdersServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>,
}

impl FinamSdk {
    /// Создает новый экземпляр клиента SDK Финам.
    ///
    /// # Аргументы
    ///
    /// * `secret` - Секретный ключ API для аутентификации в API Финам.
    ///
    /// # Возвращает
    ///
    /// * `Result<Self, FinamSdkError>` - Экземпляр SDK при успешном создании или ошибку.
    ///
    /// # Пример
    ///
    /// ```
    /// let sdk = FinamSdk::new("your_secret_key").await?;
    /// ```
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

    /// Возвращает клиент для работы со счетами.
    ///
    /// Предоставляет доступ к сервису счетов API Финам для получения информации
    /// о счетах пользователя, балансах и других операциях.
    ///
    /// # Возвращает
    ///
    /// * Клиент `AccountsServiceClient` для взаимодействия с API счетов Финам.
    pub fn accounts(
        &self,
    ) -> AccountsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.accounts.clone()
    }

    /// Возвращает клиент для работы с активами.
    ///
    /// Предоставляет доступ к сервису активов API Финам для получения информации
    /// о доступных инструментах, ценных бумагах и других активах.
    ///
    /// # Возвращает
    ///
    /// * Клиент `AssetsServiceClient` для взаимодействия с API активов Финам.
    pub fn assets(&self) -> AssetsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.assets.clone()
    }

    /// Возвращает клиент для работы с аутентификацией.
    ///
    /// Предоставляет доступ к сервису аутентификации API Финам для обновления
    /// токенов и управления сессиями.
    ///
    /// # Возвращает
    ///
    /// * Клиент `AuthServiceClient` для взаимодействия с API аутентификации Финам.
    pub fn auth(&self) -> AuthServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.auth.clone()
    }

    /// Возвращает клиент для работы с рыночными данными.
    ///
    /// Предоставляет доступ к сервису рыночных данных API Финам для получения
    /// котировок, свечей, стаканов и другой рыночной информации.
    ///
    /// # Возвращает
    ///
    /// * Клиент `MarketDataServiceClient` для взаимодействия с API рыночных данных Финам.
    pub fn market_data(
        &self,
    ) -> MarketDataServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.market_data.clone()
    }

    /// Возвращает клиент для работы с ордерами.
    ///
    /// Предоставляет доступ к сервису ордеров API Финам для создания, отмены
    /// и получения информации о торговых поручениях.
    ///
    /// # Возвращает
    ///
    /// * Клиент `OrdersServiceClient` для взаимодействия с API ордеров Финам.
    pub fn orders(&self) -> OrdersServiceClient<InterceptedService<Channel, FinamSdkInterceptor>> {
        self.orders.clone()
    }
}

/// Интерцептор для автоматического добавления JWT токена к запросам API Финам.
///
/// Отвечает за управление JWT токеном, его периодическое обновление и
/// добавление к каждому исходящему запросу в API.
#[derive(Debug, Clone)]
pub struct FinamSdkInterceptor {
    jwt_token: Arc<Mutex<String>>,
}

impl FinamSdkInterceptor {
    /// Создает новый экземпляр интерцептора SDK Финам.
    ///
    /// Генерирует JWT токен и настраивает фоновое задание для его периодического обновления.
    ///
    /// # Аргументы
    ///
    /// * `secret` - Секретный ключ API для аутентификации в API Финам.
    /// * `channel` - gRPC канал для коммуникации с API Финам.
    ///
    /// # Возвращает
    ///
    /// * `Result<Self, FinamSdkError>` - Экземпляр интерцептора при успешном создании или ошибку.
    pub async fn new(secret: &str, channel: Channel) -> Result<Self, FinamSdkError> {
        let token = Arc::new(Mutex::new(
            generate_jwt_token(channel.clone(), secret.to_string()).await?,
        ));

        let secret = secret.to_string();
        let updating_token = token.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;

                loop {
                    match generate_jwt_token(channel.clone(), secret.clone()).await {
                        Ok(value) => {
                            let token = updating_token.clone();
                            *token.lock().unwrap() = value;

                            break;
                        }

                        Err(error) => {
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            log::error!(
                                "Failed to generate JWT token. Waiting for 5 seconds... {:?}",
                                error
                            );
                        }
                    };
                }
            }
        });

        Ok(Self {
            jwt_token: token.clone(),
        })
    }

    /// Получает текущий JWT токен для авторизации.
    ///
    /// # Возвращает
    ///
    /// * `Result<String, tonic::Status>` - JWT токен при успешном получении или ошибку.
    pub fn get_jwt_token(&self) -> Result<String, tonic::Status> {
        Ok(self
            .jwt_token
            .lock()
            .map_err(|_| tonic::Status::internal("Can't lock JWT token mutex"))?
            .clone())
    }
}

/// Реализация трейта Interceptor для добавления JWT токена к запросам.
impl Interceptor for FinamSdkInterceptor {
    /// Добавляет JWT токен в заголовок авторизации к каждому исходящему запросу.
    ///
    /// # Аргументы
    ///
    /// * `request` - Исходящий gRPC запрос.
    ///
    /// # Возвращает
    ///
    /// * `Result<tonic::Request<()>, tonic::Status>` - Модифицированный запрос с токеном или ошибку.
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

/// Генерирует новый JWT токен для авторизации в API Финам.
///
/// Выполняет запрос к сервису аутентификации API Финам для получения нового JWT токена.
///
/// # Аргументы
///
/// * `channel` - gRPC канал для коммуникации с API Финам.
/// * `secret` - Секретный ключ API для аутентификации.
///
/// # Возвращает
///
/// * `Result<String, FinamSdkError>` - JWT токен при успешной генерации или ошибку.
async fn generate_jwt_token(channel: Channel, secret: String) -> Result<String, FinamSdkError> {
    let mut auth_service_client = AuthServiceClient::new(channel);
    let response = auth_service_client
        .auth(AuthRequest { secret })
        .await?
        .into_inner();

    Ok(response.token)
}

/// Ошибки, которые могут возникнуть при работе с SDK Финам.
///
/// Включает в себя ошибки транспортного уровня, ошибки статуса gRPC
/// и ошибки метаданных.
#[derive(Error, Debug)]
pub enum FinamSdkError {
    /// Ошибка транспортного уровня при коммуникации с API Финам.
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),

    /// Ошибка статуса gRPC, возвращенная API Финам.
    #[error(transparent)]
    Status(#[from] tonic::Status),

    /// Ошибка при создании или обработке метаданных запроса.
    #[error(transparent)]
    InvalidMetadataValue(#[from] InvalidMetadataValue),
}
