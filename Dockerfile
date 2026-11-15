# syntax=docker/dockerfile:1

FROM rust:1.99-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bins

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 65532 --no-create-home --shell /usr/sbin/nologin nusapeta-place
COPY --from=builder /src/target/release/nusa-place-build /usr/local/bin/
COPY --from=builder /src/target/release/nusa-place /usr/local/bin/
USER 65532:65532
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["curl", "-fsS", "http://127.0.0.1:8090/healthz"]
ENTRYPOINT ["/usr/local/bin/nusa-place"]
CMD ["--index", "/data/places.idx", "--bind", "0.0.0.0:8090"]
