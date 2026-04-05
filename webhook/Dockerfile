FROM rust:1.94-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
RUN cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates wget
COPY --from=builder /build/target/release/bitswell-webhook /usr/local/bin/
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:3000/health || exit 1
EXPOSE 3000
USER nobody
CMD ["bitswell-webhook"]
