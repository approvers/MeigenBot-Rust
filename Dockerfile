FROM rust:1.55 as base
WORKDIR /app
RUN rustup component add rustfmt
RUN cargo install cargo-chef

FROM base as planner
COPY . .
RUN cargo chef prepare --recipe-path /recipe.json

FROM base as builder
COPY --from=planner /recipe.json recipe.json

RUN cargo chef cook \
        --release \
        --recipe-path recipe.json \
        --features mongodb_,discord_webhook,api_graphql,api_grpc

COPY . .
RUN cargo build --release \
        --no-default-features \
        --features mongodb_,discord_webhook,api_graphql,api_grpc \
        --bin discord_webhook \
        --bin http_api \
        --bin grpc_api
RUN bash -c "cp /app/target/release/{discord_webhook,http_api,grpc_api} /"

FROM gcr.io/distroless/cc as discord_webhook
COPY --from=builder /app/target/release/discord_webhook /usr/local/bin/
CMD ["/usr/local/bin/discord_webhook"]

FROM gcr.io/distroless/cc as http_api
COPY --from=builder /app/target/release/http_api /usr/local/bin/
CMD ["/usr/local/bin/http_api"]

FROM gcr.io/distroless/cc as grpc_api
COPY --from=builder /app/target/release/grpc_api /usr/local/bin/
CMD ["/usr/local/bin/grpc_api"]
