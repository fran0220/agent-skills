FROM rust:latest AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/ai-search /usr/local/bin/
EXPOSE 6900
ENTRYPOINT ["ai-search"]
CMD ["serve", "--port", "6900"]
