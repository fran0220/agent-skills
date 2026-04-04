FROM rust:latest AS builder
WORKDIR /build
COPY . .
RUN cargo build --release && strip target/release/cognee-admin

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/cognee-admin /usr/local/bin/
EXPOSE 9847
ENTRYPOINT ["cognee-admin"]
CMD ["serve"]
