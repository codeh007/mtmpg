FROM rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc AS build

ARG SOURCE_REVISION

ENV DEBIAN_FRONTEND=noninteractive
ENV PGRX_PG_CONFIG_PATH=/opt/postgresql-18.4/bin/pg_config

RUN rustup toolchain install 1.97.1 \
      --profile minimal \
      --component clippy,rustfmt \
      --target x86_64-unknown-linux-gnu \
    && rustup default 1.97.1 \
    && test "$(rustc --version)" = "rustc 1.97.1 (8bab26f4f 2026-07-14)" \
    && test "$(cargo --version)" = "cargo 1.97.1 (c980f4866 2026-06-30)"

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
      bison \
      build-essential \
      bzip2 \
      ca-certificates \
      clang \
      curl \
      flex \
      jq \
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
    && build_jobs="$(nproc)" \
    && if test "${build_jobs}" -gt 4; then build_jobs=4; fi \
    && make -j"${build_jobs}" \
    && make install \
    && test "$(/opt/postgresql-18.4/bin/pg_config --version)" = "PostgreSQL 18.4"

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs rust-toolchain.toml LICENSE ./
COPY scripts/build-metadata ./scripts/build-metadata
COPY src ./src

RUN cargo build --locked --release --lib --no-default-features --features pg18 \
    && scripts/build-metadata create \
      target/release/build \
      target/release/libpggomtm.so \
      LICENSE \
      /tmp/pggomtm-build-manifest.json \
      "${SOURCE_REVISION}"

FROM postgres:18.4-bookworm@sha256:1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296

COPY --from=build --chown=0:0 --chmod=0644 /src/target/release/libpggomtm.so /usr/lib/postgresql/18/lib/pggomtm.so
COPY --from=build --chown=0:0 --chmod=0644 /src/LICENSE /usr/share/doc/pggomtm/LICENSE
COPY --from=build --chown=0:0 --chmod=0644 /tmp/pggomtm-build-manifest.json /usr/share/doc/pggomtm/build-manifest.json
