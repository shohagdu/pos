# syntax=docker/dockerfile:1.7
# Builds both binaries; select with the container command (pos-api | pos-worker).
FROM rust:1.94-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates ./crates
COPY migrations ./migrations
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p pos-api -p pos-worker \
    && mkdir -p /out && cp target/release/pos-api target/release/pos-worker /out/

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /out/ /usr/local/bin/
ENV APP_ENV=production LOG_FORMAT=json HTTP_HOST=0.0.0.0 HTTP_PORT=8080
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/pos-api"]
CMD ["serve"]
