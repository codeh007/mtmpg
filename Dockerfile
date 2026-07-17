FROM rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc AS build

ENV DEBIAN_FRONTEND=noninteractive
ENV PGRX_PG_CONFIG_PATH=/opt/postgresql-18.4/bin/pg_config

RUN rustup toolchain install 1.97.1 \
      --profile minimal \
      --component clippy,rustfmt \
      --target x86_64-unknown-linux-gnu \
    && rustup default 1.97.1 \
    && test "$(rustc --version)" = "rustc 1.97.1 (8bab26f4f 2026-07-14)" \
    && test "$(cargo --version)" = "cargo 1.97.1 (c980f4866 2026-06-30)" \
    && rustc -Vv \
    && cargo -V

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
      bison \
      build-essential \
      bzip2 \
      ca-certificates \
      clang \
      curl \
      flex \
      libclang-dev \
      libssl-dev \
      pkg-config \
      zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /tmp/postgresql-build
RUN curl --fail --location --proto '=https' --tlsv1.2 \
      --output postgresql-18.4.tar.bz2 \
      https://ftp.postgresql.org/pub/source/v18.4/postgresql-18.4.tar.bz2 \
    && echo "81a81ec695fb0c7901407defaa1d2f7973617154cf27ba74e3a7ab8e64436094  postgresql-18.4.tar.bz2" \
      | sha256sum --check --strict \
    && tar --extract --bzip2 --file postgresql-18.4.tar.bz2 \
    && cd postgresql-18.4 \
    && ./configure \
      --prefix=/opt/postgresql-18.4 \
      --with-openssl \
      --without-icu \
      --without-readline \
    && make -j2 \
    && make install \
    && test "$(/opt/postgresql-18.4/bin/pg_config --version)" = "PostgreSQL 18.4"

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml Dockerfile ./
COPY src ./src
COPY tests ./tests

RUN cc -std=c11 -Wall -Wextra -Werror \
      $(/opt/postgresql-18.4/bin/pg_config --cppflags) \
      -I/opt/postgresql-18.4/include/server \
      tests/oauth_layout_probe.c \
      -o /tmp/pggomtm_oauth_layout_probe \
    && /tmp/pggomtm_oauth_layout_probe \
    && echo "be015ae68deef28a906c8739bc653ca90a4c6966c10f0efd3bd926efb4958bcf  /opt/postgresql-18.4/include/server/libpq/oauth.h" \
      | sha256sum --check --strict

RUN cargo test --locked --no-default-features --features pg18,abi-gate \
      --test oauth_build_provenance \
      -- \
      --ignored \
      --exact real_generator_rejects_unapproved_provenance_inputs
RUN cargo test --locked --no-default-features --features pg18,abi-gate
RUN cargo test --locked --no-default-features --features pg18,abi-gate,pgx-oauth-gate \
      --test artifact_identity \
      --test pgx_oauth_gate
RUN cargo test --locked --no-default-features --features pg18,abi-runtime-gate \
      --test artifact_identity \
    && cargo build --locked --release --no-default-features --features pg18,abi-runtime-gate \
    && cp target/release/libpggomtm.so /tmp/pggomtm_abi_gate.so \
    && grep --binary-files=text --fixed-strings --quiet \
      '"features":["pg18","abi-runtime-gate"]' \
      /tmp/pggomtm_abi_gate.so \
    && cc -std=c11 -Wall -Wextra -Werror -fPIC -shared \
      $(/opt/postgresql-18.4/bin/pg_config --cppflags) \
      -I/opt/postgresql-18.4/include/server \
      tests/oauth_runtime_probe.c \
      -o /tmp/pggomtm_abi_runtime_probe.so

RUN cargo test --locked --no-default-features --features pg18 \
      --test artifact_identity \
    && cargo build --locked --release --no-default-features --features pg18 \
    && test -r target/release/libpggomtm.so \
    && grep --binary-files=text --fixed-strings --quiet \
      '"features":["pg18"]' \
      target/release/libpggomtm.so \
    && nm -D target/release/libpggomtm.so \
      | grep --quiet ' _PG_oauth_validator_module_init$'

RUN cargo fmt --check \
    && cargo clippy \
      --locked \
      --all-targets \
      --no-default-features \
      --features pg18,abi-gate \
      -- \
      -D warnings \
    && cargo clippy \
      --locked \
      --all-targets \
      --no-default-features \
      --features pg18,abi-gate,pgx-oauth-gate \
      -- \
      -D warnings \
    && cargo clippy \
      --locked \
      --lib \
      --no-default-features \
      --features pg18,abi-runtime-gate \
      -- \
      -D warnings \
    && cargo clippy \
      --locked \
      --lib \
      --no-default-features \
      --features pg18 \
      -- \
      -D warnings

FROM build AS pgx-oauth-gate-build

RUN cargo test --locked --no-default-features --features pg18,pgx-oauth-gate \
      --test artifact_identity \
    && cargo build --locked --release --no-default-features --features pg18,pgx-oauth-gate \
    && cp target/release/libpggomtm.so /tmp/pggomtm_pgx_gate.so \
    && grep --binary-files=text --fixed-strings --quiet \
      '"features":["pg18","pgx-oauth-gate"]' \
      /tmp/pggomtm_pgx_gate.so \
    && nm -D /tmp/pggomtm_pgx_gate.so \
      | grep --quiet ' _PG_oauth_validator_module_init$'

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296 AS pgx-oauth-gate

COPY --from=pgx-oauth-gate-build /tmp/pggomtm_pgx_gate.so /usr/lib/postgresql/18/lib/pggomtm_pgx_gate.so

RUN test -r /usr/lib/postgresql/18/lib/pggomtm_pgx_gate.so \
    && ! ldd /usr/lib/postgresql/18/lib/pggomtm_pgx_gate.so \
      | grep --quiet libcurl

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296 AS abi-runtime-gate

COPY --from=build /tmp/pggomtm_abi_gate.so /usr/lib/postgresql/18/lib/pggomtm_abi_gate.so
COPY --from=build /tmp/pggomtm_abi_runtime_probe.so /usr/lib/postgresql/18/lib/pggomtm_abi_runtime_probe.so
COPY --from=build /src/tests/oauth_runtime_probe.sql /tmp/oauth_runtime_probe.sql

RUN mkdir --mode=0700 /tmp/pggomtm-abi-pgdata \
    && chown postgres:postgres /tmp/pggomtm-abi-pgdata \
    && gosu postgres initdb \
      --pgdata=/tmp/pggomtm-abi-pgdata \
      --encoding=UTF8 \
      --no-locale \
      --auth-local=trust \
      --auth-host=reject \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-abi-pgdata \
      --options="-c listen_addresses='' -k /tmp" \
      --wait start \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --file=/tmp/oauth_runtime_probe.sql \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-abi-pgdata \
      --mode=fast \
      --wait stop \
    && rm -rf \
      /tmp/pggomtm-abi-pgdata \
      /tmp/oauth_runtime_probe.sql \
      /usr/lib/postgresql/18/lib/pggomtm_abi_gate.so \
      /usr/lib/postgresql/18/lib/pggomtm_abi_runtime_probe.so \
    && touch /tmp/pggomtm-abi-runtime-gate-passed

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296

COPY --from=abi-runtime-gate /tmp/pggomtm-abi-runtime-gate-passed /tmp/pggomtm-abi-runtime-gate-passed
COPY --from=build /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm.so

RUN test -r /usr/lib/postgresql/18/lib/pggomtm.so \
    && test -f /tmp/pggomtm-abi-runtime-gate-passed \
    && test ! -e /usr/lib/postgresql/18/lib/pggomtm_abi_gate.so \
    && test ! -e /usr/lib/postgresql/18/lib/pggomtm_abi_runtime_probe.so \
    && ! grep --binary-files=text --quiet 'pggomtm-abi-allocator' \
      /usr/lib/postgresql/18/lib/pggomtm.so \
    && ! grep --binary-files=text --quiet 'candidate-es256-pgx-gate' \
      /usr/lib/postgresql/18/lib/pggomtm.so \
    && ! grep --binary-files=text --quiet 'usr_pgx_gate' \
      /usr/lib/postgresql/18/lib/pggomtm.so \
    && ! ldd /usr/lib/postgresql/18/lib/pggomtm.so \
      | grep --quiet libcurl \
    && rm /tmp/pggomtm-abi-runtime-gate-passed
