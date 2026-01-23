# Finam API SDK

Версия API: [2.11.0 (23.01.2026)](https://github.com/FinamWeb/finam-trade-api/releases/tag/Release-2.11.0)

Документация: [https://tradeapi.finam.ru/docs/about/](https://tradeapi.finam.ru/docs/about/)

## Пример

```rust
async fn main() {
    let secret = env::var("TOKEN").unwrap();
    let finam = finam::FinamSdk::new(&secret).await.unwrap();

    let quote_response = finam
        .market_data()
        .last_quote(QuoteRequest {
            symbol: "SBER@MISX".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(quote) = quote_response.quote {
        println!("{:?} {:?}", quote.timestamp, quote.last);
    }

    let mut streaming = finam
        .market_data()
        .subscribe_latest_trades(SubscribeLatestTradesRequest {
            symbol: "SBER@MISX".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    loop {
        if let Some(message) = streaming.message().await.unwrap() {
            println!("{:?}", message.trades);
        }
    }
}
```

## Управление ресурсами

SDK автоматически управляет жизненным циклом JWT токенов. При создании экземпляра `FinamSdk` запускается фоновая задача, которая периодически обновляет токен каждые 10 минут.

### Автоматическая остановка фоновых задач

Когда экземпляр `FinamSdk` уничтожается (выходит из области видимости), все связанные с ним фоновые задачи обновления токенов автоматически останавливаются:

```rust
async fn example() {
    {
        let sdk = FinamSdk::new("secret").await.unwrap();
        // Использование SDK...
    } // SDK уничтожается здесь, фоновая задача остановлена

    // Фоновая задача больше не выполняется
}
```
