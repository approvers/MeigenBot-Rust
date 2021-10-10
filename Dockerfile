FROM rust:1.55 as base

COPY . /app/
WORKDIR /app
RUN cargo build --release --no-default-features --features mongodb_,discord_webhook --bin discord_webhook

FROM gcr.io/distroless/cc
COPY --from=base /app/target/release/discord_webhook /usr/local/bin
CMD ["/usr/local/bin/discord_webhook"]
