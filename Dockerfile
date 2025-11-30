# Builder stage
FROM rust:1.77-alpine as builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev

# Create a dummy project to cache dependencies
RUN cargo init
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN rm src/*.rs

# Copy source code
COPY src ./src

# Build the actual application
# Touch main.rs to ensure rebuild
RUN touch src/main.rs
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/pingap-docker-provider /usr/local/bin/pingap-docker-provider

# Install ca-certificates for HTTPS
RUN apk add --no-cache ca-certificates

# Set environment variables
ENV RUST_LOG=info

ENTRYPOINT ["pingap-docker-provider"]
