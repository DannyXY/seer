FROM rust:stable AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/seer-api /app/seer-api

ENV PORT=10000
EXPOSE 10000

CMD ["/app/seer-api"]
