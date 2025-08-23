# Finam API SDK

Документация: [https://tradeapi.finam.ru/docs/about/](https://tradeapi.finam.ru/docs/about/)

## Пример

```rust
async fn main() {
    let secret = env::var("TOKEN").unwrap();

    let finam = finam::FinamSdk::new(&secret).await.unwrap();

    let mut market_data = finam.market_data();

    let response = market_data
        .last_quote(QuoteRequest {
            symbol: "SBER@MISX".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(quote) = response.quote {
        println!("{:?} {:?}", quote.timestamp, quote.last);
    }

    let request = SubscribeLatestTradesRequest {
        symbol: "SBER@MISX".to_string(),
    };

    let response = finam
        .market_data()
        .subscribe_latest_trades(request)
        .await
        .unwrap();

    let mut streaming = response.into_inner();

    loop {
        if let Some(next_message) = streaming.message().await.unwrap() {
            println!("{:?}", next_message.trades);
        }
    }
}
```
