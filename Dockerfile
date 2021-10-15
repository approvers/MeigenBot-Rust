FROM rust:1.55 as base

COPY . /app/
WORKDIR /app

RUN rustup component add rustfmt

RUN --mount=type=cache,target=/root/.cargo/ \
    --mount=type=cache,target=/app/target \
    cargo build --release \
        --no-default-features \
        --features mongodb_,discord_webhook,api_graphql,api_grpc \
        --bin discord_webhook \
        --bin http_api \
        --bin grpc_api && \
    bash -c "cp /app/target/release/{discord_webhook,http_api,grpc_api} /"

FROM gcr.io/distroless/cc as discord_webhook
COPY --from=base / /usr/local/bin
CMD ["/usr/local/bin/discord_webhook"]

FROM gcr.io/distroless/cc as http_api
COPY --from=base / /usr/local/bin
CMD ["/usr/local/bin/http_api"]

FROM gcr.io/distroless/cc as grpc_api
COPY --from=base / /usr/local/bin
CMD ["/usr/local/bin/grpc_api"]
