# Finam API SDK

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
