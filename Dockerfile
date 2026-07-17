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

RUN curl --fail --location --proto '=https' --tlsv1.2 \
      --output /tmp/cargo-deny.tar.gz \
      https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz \
    && echo "9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f  /tmp/cargo-deny.tar.gz" \
      | sha256sum --check --strict \
    && tar --extract --gzip --file /tmp/cargo-deny.tar.gz --directory /tmp \
    && install --mode=0755 \
      /tmp/cargo-deny-0.20.2-x86_64-unknown-linux-musl/cargo-deny \
      /usr/local/bin/cargo-deny \
    && test "$(cargo-deny --version)" = "cargo-deny 0.20.2" \
    && rm -rf \
      /tmp/cargo-deny.tar.gz \
      /tmp/cargo-deny-0.20.2-x86_64-unknown-linux-musl

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml deny.toml Dockerfile ./
COPY examples ./examples
COPY src ./src
COPY tests ./tests

RUN cargo deny \
      --locked \
      --no-default-features \
      --features pg18 \
      check \
      --show-stats \
      advisories licenses bans sources

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

RUN cargo tree --locked --no-default-features --features pg18 \
      --edges normal \
      --prefix none \
      > /tmp/pggomtm-normal-dependencies.txt \
    && PGGOMTM_NORMAL_DEPENDENCY_TREE=/tmp/pggomtm-normal-dependencies.txt \
      PGGOMTM_PRODUCTION_SOURCE_ROOT=/src/src \
      PGGOMTM_PRODUCTION_ARTIFACT=/src/target/release/libpggomtm.so \
      cargo test --locked --no-default-features --features pg18,abi-gate \
        --test production_capability_gate \
        -- \
        --ignored \
    && rm /tmp/pggomtm-normal-dependencies.txt

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
      | grep --quiet ' _PG_oauth_validator_module_init$' \
    && cargo build --locked --release --example pggomtm_oauth_smoke_fixture \
      --no-default-features --features pg18,pgx-oauth-gate \
    && cp \
      target/release/examples/pggomtm_oauth_smoke_fixture \
      /tmp/pggomtm_oauth_smoke_fixture \
    && cc -std=c11 -Wall -Wextra -Werror \
      -I/opt/postgresql-18.4/include \
      tests/oauth_smoke_client.c \
      -L/opt/postgresql-18.4/lib \
      -lpq \
      -o /tmp/pggomtm_oauth_smoke_client

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296 AS pgx-oauth-gate

COPY --from=pgx-oauth-gate-build /tmp/pggomtm_pgx_gate.so /usr/lib/postgresql/18/lib/pggomtm_pgx_gate.so
COPY --from=pgx-oauth-gate-build /tmp/pggomtm_oauth_smoke_client /tmp/pggomtm_oauth_smoke_client
COPY --from=pgx-oauth-gate-build /tmp/pggomtm_oauth_smoke_fixture /tmp/pggomtm_oauth_smoke_fixture

RUN test -r /usr/lib/postgresql/18/lib/pggomtm_pgx_gate.so \
    && ! ldd /usr/lib/postgresql/18/lib/pggomtm_pgx_gate.so \
      | grep --quiet libcurl \
    && mkdir --mode=0700 /tmp/pggomtm-oauth-pgdata \
    && chown postgres:postgres /tmp/pggomtm-oauth-pgdata \
    && gosu postgres initdb \
      --pgdata=/tmp/pggomtm-oauth-pgdata \
      --encoding=UTF8 \
      --no-locale \
      --auth-local=trust \
      --auth-host=reject \
    && sed -i \
      '1ilocal all gomtm_candidate_ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm_pgx_gate delegate_ident_mapping=1' \
      /tmp/pggomtm-oauth-pgdata/pg_hba.conf \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-oauth-pgdata \
      --options="-c listen_addresses='' -k /tmp -c oauth_validator_libraries=pggomtm_pgx_gate" \
      --wait start \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --command='CREATE ROLE gomtm_candidate_ordinary LOGIN' \
    && mkdir --mode=0700 /tmp/pggomtm-oauth-fixtures \
    && /tmp/pggomtm_oauth_smoke_fixture \
      generate \
      /tmp/pggomtm-oauth-fixtures \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-oauth-fixtures/oauth-ordinary.jwt \
      gomtm_candidate_ordinary \
      /tmp/pggomtm-oauth-fixtures/oauth-ordinary.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture \
      verify-system-user \
      oauth-ordinary \
      /tmp/pggomtm-oauth-fixtures/oauth-ordinary.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture \
      verify-codec-rejections \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-rejected \
      /tmp/pggomtm-oauth-fixtures/tampered.jwt \
      gomtm_candidate_ordinary \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-oauth-pgdata \
      --mode=fast \
      --wait stop \
    && rm -rf \
      /tmp/pggomtm-oauth-pgdata \
      /tmp/pggomtm-oauth-fixtures \
      /tmp/pggomtm_oauth_smoke_client \
      /tmp/pggomtm_oauth_smoke_fixture \
    && touch /tmp/pggomtm-oauth-smoke-passed

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296 AS abi-runtime-gate

COPY --from=build /tmp/pggomtm_abi_gate.so /usr/lib/postgresql/18/lib/pggomtm_abi_gate.so
COPY --from=build /tmp/pggomtm_abi_runtime_probe.so /usr/lib/postgresql/18/lib/pggomtm_abi_runtime_probe.so
COPY --from=build /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm_config_gate.so
COPY --from=build /src/tests/oauth_runtime_probe.sql /tmp/oauth_runtime_probe.sql
COPY --from=build /src/tests/runtime_config_missing_probe.sql /tmp/runtime_config_missing_probe.sql
COPY --from=build /src/tests/runtime_config_ready_probe.sql /tmp/runtime_config_ready_probe.sql
COPY --from=build /src/tests/runtime_config_validate_probe.sql /tmp/runtime_config_validate_probe.sql
COPY --from=build /src/tests/fixtures/runtime-config /tmp/runtime-config-fixture
COPY --from=pgx-oauth-gate-build /tmp/pggomtm_oauth_smoke_fixture /tmp/pggomtm_oauth_smoke_fixture

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
      --log=/tmp/pggomtm-abi-server.log \
      --options="-c listen_addresses='' -k /tmp -c log_min_messages=log" \
      --wait start \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --file=/tmp/oauth_runtime_probe.sql \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --file=/tmp/runtime_config_missing_probe.sql \
    && mkdir --mode=0555 /etc/pggomtm \
    && install --mode=0444 \
      /tmp/runtime-config-fixture/validator.json \
      /etc/pggomtm/validator.json \
    && install --mode=0444 \
      /tmp/runtime-config-fixture/jwks.json \
      /etc/pggomtm/jwks.json \
    && mkdir --mode=0700 /tmp/pggomtm-config-fixtures \
    && /tmp/pggomtm_oauth_smoke_fixture \
      generate \
      /tmp/pggomtm-config-fixtures \
    && chown --recursive postgres:postgres /tmp/pggomtm-config-fixtures \
    && chmod 0400 /tmp/pggomtm-config-fixtures/*.jwt \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --file=/tmp/runtime_config_ready_probe.sql \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --file=/tmp/runtime_config_validate_probe.sql \
    && grep --extended-regexp --quiet \
      'LOG:.*pggomtm authentication rejected: reason=pggomtm-auth/v1/internal-panic' \
      /tmp/pggomtm-abi-server.log \
    && ! grep --extended-regexp --quiet \
      'stack backtrace|panicked at|RUST_BACKTRACE|src/lib\.rs|eyJ[A-Za-z0-9_-]+\.' \
      /tmp/pggomtm-abi-server.log \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-abi-pgdata \
      --mode=fast \
      --wait stop \
    && rm -rf \
      /tmp/pggomtm-abi-pgdata \
      /tmp/oauth_runtime_probe.sql \
      /tmp/runtime_config_missing_probe.sql \
      /tmp/runtime_config_ready_probe.sql \
      /tmp/runtime_config_validate_probe.sql \
      /tmp/pggomtm-abi-server.log \
      /tmp/runtime-config-fixture \
      /tmp/pggomtm-config-fixtures \
      /tmp/pggomtm_oauth_smoke_fixture \
      /etc/pggomtm \
      /usr/lib/postgresql/18/lib/pggomtm_abi_gate.so \
      /usr/lib/postgresql/18/lib/pggomtm_abi_runtime_probe.so \
      /usr/lib/postgresql/18/lib/pggomtm_config_gate.so \
    && touch /tmp/pggomtm-abi-runtime-gate-passed

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296 AS production-identity-gate

COPY --from=build /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm_identity_gate.so
COPY --from=build /src/tests/fixtures/runtime-config /tmp/runtime-config-fixture
COPY --from=pgx-oauth-gate-build /tmp/pggomtm_oauth_smoke_client /tmp/pggomtm_oauth_smoke_client
COPY --from=pgx-oauth-gate-build /tmp/pggomtm_oauth_smoke_fixture /tmp/pggomtm_oauth_smoke_fixture

RUN mkdir --mode=0700 /tmp/pggomtm-production-identity-pgdata \
    && chown postgres:postgres /tmp/pggomtm-production-identity-pgdata \
    && gosu postgres initdb \
      --pgdata=/tmp/pggomtm-production-identity-pgdata \
      --encoding=UTF8 \
      --no-locale \
      --auth-local=trust \
      --auth-host=reject \
    && sed -i \
      '1ilocal all gomtm_candidate_ordinary oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm_identity_gate delegate_ident_mapping=1' \
      /tmp/pggomtm-production-identity-pgdata/pg_hba.conf \
    && sed -i \
      '1ilocal all gomtm_candidate_business_admin oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm_identity_gate delegate_ident_mapping=1' \
      /tmp/pggomtm-production-identity-pgdata/pg_hba.conf \
    && sed -i \
      '1ilocal all gomtm_candidate_database_developer oauth issuer="https://candidate.example.test/oauth/database" scope="database" validator=pggomtm_identity_gate delegate_ident_mapping=1' \
      /tmp/pggomtm-production-identity-pgdata/pg_hba.conf \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-production-identity-pgdata \
      --log=/tmp/pggomtm-production-identity-server.log \
      --options="-c listen_addresses='' -k /tmp -c log_min_messages=log -c oauth_validator_libraries=pggomtm_identity_gate" \
      --wait start \
    && gosu postgres psql \
      --host=/tmp \
      --username=postgres \
      --dbname=postgres \
      --command='CREATE ROLE gomtm_candidate_ordinary LOGIN; CREATE ROLE gomtm_candidate_business_admin LOGIN; CREATE ROLE gomtm_candidate_database_developer LOGIN' \
    && mkdir --mode=0700 /tmp/pggomtm-production-identity-fixtures \
    && /tmp/pggomtm_oauth_smoke_fixture \
      generate \
      /tmp/pggomtm-production-identity-fixtures \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-startup-rejected \
      /tmp/pggomtm-production-identity-fixtures/oauth-ordinary.jwt \
      gomtm_candidate_ordinary \
    && mkdir --mode=0555 /etc/pggomtm \
    && install --mode=0444 \
      /tmp/runtime-config-fixture/validator.json \
      /etc/pggomtm/validator.json \
    && install --mode=0444 \
      /tmp/runtime-config-fixture/jwks.json \
      /etc/pggomtm/jwks.json \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-production-identity-fixtures/oauth-ordinary.jwt \
      gomtm_candidate_ordinary \
      /tmp/pggomtm-production-identity-fixtures/oauth-ordinary.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture verify-system-user oauth-ordinary \
      /tmp/pggomtm-production-identity-fixtures/oauth-ordinary.system-user \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-production-identity-fixtures/oauth-business-admin.jwt \
      gomtm_candidate_business_admin \
      /tmp/pggomtm-production-identity-fixtures/oauth-business-admin.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture verify-system-user oauth-business-admin \
      /tmp/pggomtm-production-identity-fixtures/oauth-business-admin.system-user \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-production-identity-fixtures/oauth-database-developer.jwt \
      gomtm_candidate_database_developer \
      /tmp/pggomtm-production-identity-fixtures/oauth-database-developer.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture verify-system-user oauth-database-developer \
      /tmp/pggomtm-production-identity-fixtures/oauth-database-developer.system-user \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-production-identity-fixtures/api-key-ordinary.jwt \
      gomtm_candidate_ordinary \
      /tmp/pggomtm-production-identity-fixtures/api-key-ordinary.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture verify-system-user api-key-ordinary \
      /tmp/pggomtm-production-identity-fixtures/api-key-ordinary.system-user \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-production-identity-fixtures/api-key-business-admin.jwt \
      gomtm_candidate_business_admin \
      /tmp/pggomtm-production-identity-fixtures/api-key-business-admin.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture verify-system-user api-key-business-admin \
      /tmp/pggomtm-production-identity-fixtures/api-key-business-admin.system-user \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-allowed \
      /tmp/pggomtm-production-identity-fixtures/api-key-database-developer.jwt \
      gomtm_candidate_database_developer \
      /tmp/pggomtm-production-identity-fixtures/api-key-database-developer.system-user \
    && /tmp/pggomtm_oauth_smoke_fixture verify-system-user api-key-database-developer \
      /tmp/pggomtm-production-identity-fixtures/api-key-database-developer.system-user \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-rejected \
      /tmp/pggomtm-production-identity-fixtures/invalid-overlong-identity.jwt \
      gomtm_candidate_ordinary \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-rejected \
      /tmp/pggomtm-production-identity-fixtures/invalid-illegal-identity.jwt \
      gomtm_candidate_ordinary \
    && /tmp/pggomtm_oauth_smoke_client \
      --expect-rejected \
      /tmp/pggomtm-production-identity-fixtures/tampered.jwt \
      gomtm_candidate_ordinary \
    && /tmp/pggomtm_oauth_smoke_fixture verify-codec-rejections \
    && gosu postgres pg_ctl \
      --pgdata=/tmp/pggomtm-production-identity-pgdata \
      --mode=fast \
      --wait stop \
    && grep --extended-regexp --quiet \
      '(ERROR|FATAL):.*pggomtm authentication failed: reason=pggomtm-auth/v1/config-missing' \
      /tmp/pggomtm-production-identity-server.log \
    && grep --extended-regexp --quiet \
      'LOG:.*pggomtm authentication rejected: reason=pggomtm-auth/v1/token-signature-invalid' \
      /tmp/pggomtm-production-identity-server.log \
    && test "$(grep --count --fixed-strings \
      'pggomtm authentication rejected: reason=pggomtm-auth/v1/identity-invalid' \
      /tmp/pggomtm-production-identity-server.log)" -ge 2 \
    && for fixture in /tmp/pggomtm-production-identity-fixtures/*.jwt; do \
      ! grep --fixed-strings --file="$fixture" \
        /tmp/pggomtm-production-identity-server.log || exit 1; \
    done \
    && ! grep --fixed-strings --file=/etc/pggomtm/validator.json \
      /tmp/pggomtm-production-identity-server.log \
    && ! grep --fixed-strings --file=/etc/pggomtm/jwks.json \
      /tmp/pggomtm-production-identity-server.log \
    && ! grep --extended-regexp --quiet \
      'Authorization: Bearer|postgres(ql)?://|stack backtrace|panicked at|RUST_BACKTRACE|src/lib\.rs|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY' \
      /tmp/pggomtm-production-identity-server.log \
    && rm -rf \
      /tmp/pggomtm-production-identity-pgdata \
      /tmp/pggomtm-production-identity-fixtures \
      /tmp/pggomtm-production-identity-server.log \
      /tmp/runtime-config-fixture \
      /tmp/pggomtm_oauth_smoke_client \
      /tmp/pggomtm_oauth_smoke_fixture \
      /etc/pggomtm \
      /usr/lib/postgresql/18/lib/pggomtm_identity_gate.so \
    && touch /tmp/pggomtm-production-identity-passed

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296

COPY --from=abi-runtime-gate /tmp/pggomtm-abi-runtime-gate-passed /tmp/pggomtm-abi-runtime-gate-passed
COPY --from=pgx-oauth-gate /tmp/pggomtm-oauth-smoke-passed /tmp/pggomtm-oauth-smoke-passed
COPY --from=production-identity-gate /tmp/pggomtm-production-identity-passed /tmp/pggomtm-production-identity-passed
COPY --from=build /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm.so

RUN test -r /usr/lib/postgresql/18/lib/pggomtm.so \
    && test -f /tmp/pggomtm-abi-runtime-gate-passed \
    && test -f /tmp/pggomtm-oauth-smoke-passed \
    && test -f /tmp/pggomtm-production-identity-passed \
    && test ! -e /usr/lib/postgresql/18/lib/pggomtm_abi_gate.so \
    && test ! -e /usr/lib/postgresql/18/lib/pggomtm_abi_runtime_probe.so \
    && test ! -e /usr/lib/postgresql/18/lib/pggomtm_config_gate.so \
    && rm \
      /tmp/pggomtm-abi-runtime-gate-passed \
      /tmp/pggomtm-oauth-smoke-passed \
      /tmp/pggomtm-production-identity-passed
