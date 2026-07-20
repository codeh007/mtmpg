ARG RUST_IMAGE=rust:bookworm
ARG POSTGRES_IMAGE=postgres:18-bookworm
FROM ${RUST_IMAGE} AS build

ARG POSTGRES_MINOR

ENV DEBIAN_FRONTEND=noninteractive
ENV PGRX_PG_CONFIG_PATH=/usr/lib/postgresql/18/bin/pg_config

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
      build-essential \
      ca-certificates \
      clang \
      curl \
      gnupg \
      libclang-dev \
      libkrb5-dev \
      libssl-dev \
      pkg-config \
    && curl --fail --location --proto '=https' --tlsv1.2 \
      https://www.postgresql.org/media/keys/ACCC4CF8.asc \
      | gpg --dearmor --output /usr/share/keyrings/postgresql.gpg \
    && echo "deb [signed-by=/usr/share/keyrings/postgresql.gpg] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main" \
      > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install --yes --no-install-recommends postgresql-server-dev-18 \
    && if test -n "${POSTGRES_MINOR}"; then \
      test "$("${PGRX_PG_CONFIG_PATH}" --version | grep --only-matching --extended-regexp '18\.[0-9]+' | head -n 1)" = "${POSTGRES_MINOR}"; \
    fi \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml LICENSE ./
COPY executor/Cargo.toml ./executor/Cargo.toml
COPY src ./src
COPY executor/src ./executor/src
COPY executor/tests/support ./executor/tests/support

RUN cargo build --locked --release --lib --no-default-features --features pg18

FROM ${POSTGRES_IMAGE}

ARG SOURCE_REVISION=unknown
ARG VERSION=0.0.0-dev

LABEL org.opencontainers.image.source="https://github.com/codeh007/mtmpg" \
      org.opencontainers.image.revision="${SOURCE_REVISION}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.licenses="MIT"

COPY --from=build --chown=0:0 --chmod=0644 /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm.so
COPY --from=build --chown=0:0 --chmod=0644 /src/LICENSE /usr/share/doc/pggomtm/LICENSE
