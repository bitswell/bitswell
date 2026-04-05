FROM rust:1.94-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
RUN cargo build --release

FROM node:22-alpine
RUN apk add --no-cache ca-certificates wget git
RUN npm install -g @anthropic-ai/claude-code
COPY --from=builder /build/target/release/bitswell-webhook /usr/local/bin/
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:3000/health || exit 1
EXPOSE 3000
CMD ["bitswell-webhook"]
