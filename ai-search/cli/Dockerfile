FROM rust:latest AS builder
WORKDIR /build
COPY . .
RUN cargo build --release && strip target/release/ai-search

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/ai-search /usr/local/bin/
EXPOSE 6900
ENTRYPOINT ["ai-search"]
CMD ["serve", "--port", "6900"]
