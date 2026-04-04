FROM golang:1.24-alpine AS builder

RUN apk add --no-cache ca-certificates tzdata git
WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /grok2api ./cmd/server

FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata
WORKDIR /app

COPY --from=builder /grok2api /grok2api
COPY config.defaults.toml /app/config.defaults.toml
COPY _public /app/_public

RUN mkdir -p /app/data /app/logs

EXPOSE 8000

ENV SERVER_HOST=0.0.0.0 \
	SERVER_PORT=8000 \
	TZ=Asia/Shanghai

CMD ["/grok2api"]
