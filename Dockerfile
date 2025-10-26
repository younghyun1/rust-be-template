ARG RUST_VERSION=1.90.0
ARG APP_NAME=rust-be-template

# --- Build Stage ---
FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install build dependencies, including tools for vendored OpenSSL
RUN apk add --no-cache clang lld musl-dev git ca-certificates postgresql-dev upx zstd-static pkgconf make perl

# Build the application, ensuring the `fe` directory is mounted for rust-embed
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=fe,target=fe \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    upx --lzma --best ./target/release/$APP_NAME && \
    cp ./target/release/$APP_NAME /bin/server

# --- Final Stage ---
FROM scratch AS final

# Copy the server executable from the build stage
COPY --from=build /bin/server /bin/

# Copy database bundle files
COPY new_bundle_ipv4.db /bin/
COPY new_bundle_ipv6.db /bin/

# Set environment variables for the application
ENV CURR_ENV="dev"
ENV HOST_IP="127.0.0.1"
ENV HOST_PORT="443"
ENV DB_URL="postgres://be_admin:!1^CnhVBB7vfSFzlQ@host.docker.internal/be_db"

# Expose the application port
EXPOSE 443

# Set the command to run the application
CMD ["/bin/server"]
