FROM rust:1.86-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/asset-gateway /usr/local/bin/
EXPOSE 6700
ENTRYPOINT ["asset-gateway"]
CMD ["serve", "--host", "0.0.0.0", "--port", "6700"]
