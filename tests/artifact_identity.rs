use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const POSTGRESQL_SOURCE_SHA256: &str =
    "81a81ec695fb0c7901407defaa1d2f7973617154cf27ba74e3a7ab8e64436094";
const OAUTH_HEADER_SHA256: &str =
    "be015ae68deef28a906c8739bc653ca90a4c6966c10f0efd3bd926efb4958bcf";
const OAUTH_BINDINGS_SHA256: &str =
    "b6f8bf810c467f74a0e43f9019f00cfd517cc881c9b606175818ca1b17204beb";
const RUNTIME_BASE_SHA256: &str =
    "1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296";

#[test]
fn build_variant_has_a_complete_comparable_artifact_identity() {
    let encoded = env!("PGGOMTM_BUILD_IDENTITY_JSON");
    let identity_sha256 = env!("PGGOMTM_BUILD_IDENTITY_SHA256");
    assert_eq!(
        format!("{:x}", Sha256::digest(encoded.as_bytes())),
        identity_sha256,
        "artifact identity digest must cover the exact canonical JSON bytes"
    );

    let identity: Value = serde_json::from_str(encoded).expect("parse artifact identity JSON");
    assert_eq!(
        identity,
        json!({
            "schema": "pggomtm-build-identity/v1",
            "module_version": env!("CARGO_PKG_VERSION"),
            "features": expected_features(),
            "rust": {
                "version": "1.97.1",
                "target": "x86_64-unknown-linux-gnu"
            },
            "dependencies": {
                "pgrx": "0.19.1",
                "jose_implementation": "jaws",
                "jose_version": "1.0.4"
            },
            "postgresql": {
                "source_version": "18.4",
                "pg_version_num": 180004,
                "source_sha256": POSTGRESQL_SOURCE_SHA256,
                "oauth_header_sha256": OAUTH_HEADER_SHA256,
                "oauth_bindings_sha256": OAUTH_BINDINGS_SHA256,
                "runtime_base": "postgres:18.4-bookworm",
                "runtime_base_sha256": RUNTIME_BASE_SHA256
            },
            "platform": {
                "os": "linux",
                "arch": "amd64",
                "libc": "glibc"
            }
        }),
        "artifact identity fields or canonical values changed"
    );

    assert_eq!(locked_version("pgrx"), "0.19.1");
    assert_eq!(locked_version("jaws"), "1.0.4");
    assert!(
        include_str!("../rust-toolchain.toml").contains("channel = \"1.97.1\""),
        "tracked Rust toolchain must match the identity"
    );
}

fn expected_features() -> Vec<&'static str> {
    let mut features = vec!["pg18"];
    if cfg!(feature = "abi-gate") {
        features.push("abi-gate");
    }
    if cfg!(feature = "abi-runtime-gate") {
        features.push("abi-runtime-gate");
    }
    if cfg!(feature = "pgx-oauth-gate") {
        features.push("pgx-oauth-gate");
    }
    features
}

fn locked_version(package: &str) -> &str {
    include_str!("../Cargo.lock")
        .split("[[package]]")
        .find(|block| {
            block
                .lines()
                .any(|line| line == format!("name = \"{package}\""))
        })
        .and_then(|block| {
            block
                .lines()
                .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        })
        .unwrap_or_else(|| panic!("locked package {package} must have one version"))
}
