ARG RUST_IMAGE=rust:bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

FROM ${RUST_IMAGE} AS build

ARG POSTGRES_MINOR

ENV CARGO_TARGET_DIR=/src/target
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
      openssl \
      pkg-config \
    && curl --fail --location --proto '=https' --tlsv1.2 \
      https://www.postgresql.org/media/keys/ACCC4CF8.asc \
      | gpg --dearmor --output /usr/share/keyrings/postgresql.gpg \
    && echo "deb [signed-by=/usr/share/keyrings/postgresql.gpg] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main" \
      > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install --yes --no-install-recommends postgresql-server-dev-18 \
    && test -n "${POSTGRES_MINOR}" \
    && test "$("${PGRX_PG_CONFIG_PATH}" --version | grep --only-matching --extended-regexp '18\.[0-9]+' | head -n 1)" = "${POSTGRES_MINOR}" \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml LICENSE ./
COPY src ./src
COPY tests/support/oauth_fixture.rs ./tests/support/oauth_fixture.rs
COPY executor/Cargo.toml executor/build.rs ./executor/
COPY executor/src ./executor/src
COPY executor/tests/postgres_setup.sql ./executor/tests/postgres_setup.sql
COPY executor/tests/stage-integration.sh ./executor/tests/stage-integration.sh
COPY executor/tests/support ./executor/tests/support

RUN cargo build --locked --release --lib --no-default-features --features pg18 \
    && install -d -m 0755 /tmp/executor-validator \
    && install -m 0644 \
      "${CARGO_TARGET_DIR}/release/libpggomtm.so" \
      /tmp/executor-validator/pggomtm.so \
    && GITHUB_ACTIONS=true executor/tests/stage-integration.sh \
      /tmp/executor-validator \
      "${CARGO_TARGET_DIR}/executor-integration"

FROM scratch AS integration-artifacts
COPY --from=build /src/target/executor-integration /

FROM ${RUNTIME_IMAGE}

ARG POSTGRES_MINOR
ARG SOURCE_REVISION=unknown
ARG VERSION=0.0.0-dev

LABEL org.opencontainers.image.source="https://github.com/codeh007/mtmpg" \
      org.opencontainers.image.revision="${SOURCE_REVISION}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.licenses="MIT"

COPY --from=build /usr/share/keyrings/postgresql.gpg /usr/share/keyrings/postgresql.gpg
RUN test -n "${POSTGRES_MINOR}" \
    && echo "deb [signed-by=/usr/share/keyrings/postgresql.gpg] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main" \
      > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates libpq5 \
    && case "$(dpkg-query --show --showformat='${Version}' libpq5)" in \
      "${POSTGRES_MINOR}"-*) ;; \
      *) exit 1 ;; \
    esac \
    && rm -rf /var/lib/apt/lists/* /etc/apt/sources.list.d/pgdg.list /usr/share/keyrings/postgresql.gpg

COPY --from=build --chown=10001:10001 --chmod=0555 \
  /src/target/release/mtmpg-executor \
  /usr/local/bin/mtmpg-executor
COPY --from=build --chown=10001:10001 --chmod=0444 \
  /src/LICENSE \
  /usr/share/doc/mtmpg-executor/LICENSE

USER 10001:10001
EXPOSE 8443
STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/local/bin/mtmpg-executor"]
