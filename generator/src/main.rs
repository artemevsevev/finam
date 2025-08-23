fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false)
        .out_dir("../src")
        .compile_protos(
            &[
                "../finam-trade-api/proto/grpc/tradeapi/v1/accounts/accounts_service.proto",
                "../finam-trade-api/proto/grpc/tradeapi/v1/assets/assets_service.proto",
                "../finam-trade-api/proto/grpc/tradeapi/v1/auth/auth_service.proto",
                "../finam-trade-api/proto/grpc/tradeapi/v1/marketdata/marketdata_service.proto",
                "../finam-trade-api/proto/grpc/tradeapi/v1/orders/orders_service.proto",
                "../finam-trade-api/proto/grpc/tradeapi/v1/side.proto",
                "../finam-trade-api/proto/grpc/tradeapi/v1/trade.proto",
            ],
            &["../finam-trade-api/proto", "../googleapis"],
        )?;

    std::fs::rename("../src/grpc.tradeapi.v1.rs", "../src/api.rs")?;
    std::fs::rename(
        "../src/grpc.tradeapi.v1.accounts.rs",
        "../src/api.accounts.rs",
    )?;
    std::fs::rename("../src/grpc.tradeapi.v1.assets.rs", "../src/api.assets.rs")?;
    std::fs::rename("../src/grpc.tradeapi.v1.auth.rs", "../src/api.auth.rs")?;
    std::fs::rename(
        "../src/grpc.tradeapi.v1.marketdata.rs",
        "../src/api.marketdata.rs",
    )?;
    std::fs::rename("../src/grpc.tradeapi.v1.orders.rs", "../src/api.orders.rs")?;

    Ok(())
}
