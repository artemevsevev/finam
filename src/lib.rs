use std::sync::{Arc, RwLock};

use thiserror::Error;
use tokio::sync::oneshot;
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

pub type FinamAccountsServiceClient =
    AccountsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>;
pub type FinamAssetsServiceClient =
    AssetsServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>;
pub type FinamAuthServiceClient =
    AuthServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>;
pub type FinamMarketDataServiceClient =
    MarketDataServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>;
pub type FinamOrdersServiceClient =
    OrdersServiceClient<InterceptedService<Channel, FinamSdkInterceptor>>;

/// Основной клиент SDK для работы с API Финам.
///
/// Предоставляет доступ к различным сервисам API Финам через gRPC.
/// Включает в себя клиенты для работы со счетами, активами, аутентификацией,
/// рыночными данными и ордерами.
#[derive(Clone, Debug)]
pub struct FinamSdk {
    accounts: FinamAccountsServiceClient,
    assets: FinamAssetsServiceClient,
    auth: FinamAuthServiceClient,
    market_data: FinamMarketDataServiceClient,
    orders: FinamOrdersServiceClient,
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
    /// ```no_run
    /// use finam::FinamSdk;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let sdk = FinamSdk::new("your_secret_key").await?;
    ///     Ok(())
    /// }
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
    pub fn accounts(&self) -> FinamAccountsServiceClient {
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
    pub fn assets(&self) -> FinamAssetsServiceClient {
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
    pub fn auth(&self) -> FinamAuthServiceClient {
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
    pub fn market_data(&self) -> FinamMarketDataServiceClient {
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
    pub fn orders(&self) -> FinamOrdersServiceClient {
        self.orders.clone()
    }
}

/// Охранник для корректного завершения фонового потока обновления токена.
///
/// Отправляет сигнал завершения при уничтожении последней ссылки на интерцептор.
struct ShutdownGuard {
    sender: Option<oneshot::Sender<()>>,
}

impl std::fmt::Debug for ShutdownGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownGuard").finish_non_exhaustive()
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(()); // Signal shutdown to the background task
        }
    }
}

/// Интерцептор для автоматического добавления JWT токена к запросам API Финам.
///
/// Отвечает за управление JWT токеном, его периодическое обновление и
/// добавление к каждому исходящему запросу в API.
#[derive(Clone, Debug)]
pub struct FinamSdkInterceptor {
    jwt_token: Arc<RwLock<String>>,
    /// Удерживает фоновый поток обновления токена. При уничтожении последней
    /// ссылки на интерцептор отправляет сигнал завершения.
    #[allow(dead_code)]
    shutdown_guard: Arc<ShutdownGuard>,
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
        let token = Arc::new(RwLock::new(
            generate_jwt_token(channel.clone(), secret.to_string()).await?,
        ));

        let secret = secret.to_string();
        let updating_token = token.clone();
        let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_receiver => {
                        log::info!("Token refresh task shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)) => {
                        // Token refresh logic
                        loop {
                            match generate_jwt_token(channel.clone(), secret.clone()).await {
                                Ok(value) => match updating_token.write() {
                                    Ok(mut token_guard) => {
                                        *token_guard = value;
                                        break;
                                    }
                                    Err(error) => {
                                        log::error!(
                                            "Failed to write JWT token. Waiting for 5 seconds... {:?}",
                                            error
                                        );
                                    }
                                },

                                Err(error) => {
                                    log::error!(
                                        "Failed to generate JWT token. Waiting for 5 seconds... {:?}",
                                        error
                                    );
                                }
                            };

                            // Check for shutdown signal during retry delay
                            tokio::select! {
                                _ = &mut shutdown_receiver => {
                                    log::info!("Token refresh task shutting down during retry");
                                    return;
                                }
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            jwt_token: token,
            shutdown_guard: Arc::new(ShutdownGuard {
                sender: Some(shutdown_sender),
            }),
        })
    }

    /// Получает текущий JWT токен для авторизации.
    ///
    /// # Возвращает
    ///
    /// * `Result<String, tonic::Status>` - JWT токен при успешном получении или ошибку.
    fn get_jwt_token(&self) -> Result<String, tonic::Status> {
        Ok(self
            .jwt_token
            .read()
            .map_err(|_| tonic::Status::internal("Can't read JWT token"))?
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_token_refresh_shutdown() {
        // Initialize logger for test
        let _ = env_logger::try_init();

        // Create a flag to track if the background task is still running
        let task_running = Arc::new(AtomicBool::new(true));
        let task_running_clone = task_running.clone();

        // Create a mock interceptor to test shutdown mechanism
        let _token = Arc::new(RwLock::new("initial_token".to_string()));
        let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();

        // Spawn the token refresh task similar to the real implementation
        let background_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_receiver => {
                        log::info!("Test token refresh task shutting down");
                        task_running_clone.store(false, Ordering::SeqCst);
                        break;
                    }
                    _ = sleep(Duration::from_millis(100)) => {
                        // Short interval for testing
                        log::debug!("Test token refresh tick");
                    }
                }
            }
        });

        // Verify task is running
        assert!(task_running.load(Ordering::SeqCst));
        sleep(Duration::from_millis(50)).await;
        assert!(task_running.load(Ordering::SeqCst));

        // Send shutdown signal
        let _ = shutdown_sender.send(());

        // Wait for task to shutdown
        let _ = background_task.await;

        // Verify task has stopped
        assert!(!task_running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_interceptor_drop_triggers_shutdown() {
        // Initialize logger for test
        let _ = env_logger::try_init();

        let task_completed = Arc::new(AtomicBool::new(false));
        let task_completed_clone = task_completed.clone();

        // Create interceptor in a scope so it gets dropped
        {
            let token = Arc::new(RwLock::new("test_token".to_string()));
            let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();

            // Spawn a task that waits for shutdown signal
            tokio::spawn(async move {
                tokio::select! {
                    _ = &mut shutdown_receiver => {
                        log::info!("Shutdown signal received in test");
                        task_completed_clone.store(true, Ordering::SeqCst);
                    }
                    _ = sleep(Duration::from_secs(10)) => {
                        // This should not happen in normal test execution
                        log::error!("Test task timed out waiting for shutdown signal");
                    }
                }
            });

            let interceptor = FinamSdkInterceptor {
                jwt_token: token,
                shutdown_guard: Arc::new(ShutdownGuard {
                    sender: Some(shutdown_sender),
                }),
            };

            // Use interceptor briefly
            sleep(Duration::from_millis(50)).await;

            // Drop interceptor - this should trigger shutdown
            drop(interceptor);
        }

        // Wait a bit for the shutdown signal to be processed
        sleep(Duration::from_millis(100)).await;

        // Verify that the shutdown signal was sent
        assert!(task_completed.load(Ordering::SeqCst));
    }
}
