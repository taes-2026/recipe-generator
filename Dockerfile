FROM rust:1-slim-bookworm AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/recipe-generator /usr/local/bin/recipe-generator

ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
USER 65534:65534
ENTRYPOINT ["recipe-generator"]
