ARG RUST_VERSION=1.84.0
ARG APP_NAME=rust-be-template

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
# Added postgresql-dev to install libpq
RUN apk add --no-cache clang lld musl-dev git ca-certificates upx postgresql-dev

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server && \
    upx --lzma --best /bin/server

FROM scratch AS final
COPY --from=build /bin/server /bin/

ENV IS_AWS=true
ENV APP_NAME_VERSION=rust-be-template-0.1.0
ENV DB_URL=postgresql://spring_learn_admin:K7ww89Sj!5@host.docker.internal:5432/spring_learn

CMD ["/bin/server"]
