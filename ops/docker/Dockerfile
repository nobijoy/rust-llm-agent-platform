FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p api

FROM debian:trixie-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY migrations /app/migrations
RUN mkdir -p /app/data
EXPOSE 8080
CMD ["api"]
