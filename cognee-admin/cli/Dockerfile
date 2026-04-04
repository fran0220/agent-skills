FROM rust:1.86-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/cognee-admin /usr/local/bin/
EXPOSE 9847
ENTRYPOINT ["cognee-admin"]
CMD ["serve"]
