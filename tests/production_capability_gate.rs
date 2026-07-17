use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ALLOWED_DYNAMIC_DEPENDENCIES: &[&str] =
    &["ld-linux-x86-64.so.2", "libc.so.6", "libgcc_s.so.1"];
const ALLOWED_DEFINED_SYMBOLS: &[&str] = &["Pg_magic_func", "_PG_oauth_validator_module_init"];
const FORBIDDEN_NORMAL_DEPENDENCIES: &[&str] = &[
    "attohttpc",
    "awc",
    "curl",
    "curl-sys",
    "diesel",
    "h2",
    "hickory-resolver",
    "http",
    "http-body",
    "http-body-util",
    "hyper",
    "hyper-util",
    "isahc",
    "libpq",
    "minreq",
    "mio",
    "mysql",
    "mysql_async",
    "postgres",
    "pq-sys",
    "reqwest",
    "rusqlite",
    "sea-orm",
    "socket2",
    "sqlx",
    "sqlx-core",
    "surf",
    "tiberius",
    "tokio",
    "tokio-postgres",
    "trust-dns-proto",
    "trust-dns-resolver",
    "ureq",
];
const FORBIDDEN_SOURCE_FRAGMENTS: &[&str] = &[
    "std::net",
    "TcpStream",
    "UdpSocket",
    "getaddrinfo",
    "gethostbyname",
    "reqwest::",
    "hyper::",
    "curl::",
    "hickory_resolver",
    "trust_dns",
    "pgrx::spi",
    "pgrx::Spi",
    "pg_sys::SPI_",
    "SPI_connect",
    "SPI_execute",
    "sqlx::",
    "tokio_postgres",
    "postgres::Client",
    "PQconnect",
    "SigningKey",
    "SecretKey",
    "from_pkcs8",
    "from_sec1",
    "private_key_path",
    "service_credential",
    "introspection_endpoint",
    "fallback_issuer",
    "secondary_issuer",
    "PGGOMTM_CONFIG_PATH",
    "std::env::var(",
    "std::env::var_os(",
];
const FORBIDDEN_UNDEFINED_SYMBOLS: &[&str] = &[
    "BIO_connect",
    "SSL_connect",
    "connect",
    "getaddrinfo",
    "gethostbyaddr",
    "gethostbyname",
    "getnameinfo",
    "recv",
    "recvfrom",
    "res_query",
    "res_search",
    "send",
    "sendto",
    "socket",
    "socketpair",
];
const FORBIDDEN_UNDEFINED_SYMBOL_PREFIXES: &[&str] =
    &["PQ", "SPI_", "ares_", "curl_", "nghttp2_", "uv_getaddrinfo"];
const FORBIDDEN_ARTIFACT_STRINGS: &[&str] = &[
    "-----begin private key-----",
    "/introspect",
    "authorization: bearer",
    "client_secret",
    "fallback_issuer",
    "http://",
    "introspection_endpoint",
    "pggomtm_config_path",
    "private_key_path",
    "secondary_issuer",
    "service_account",
    "service_credential",
];
const FORBIDDEN_GATE_ARTIFACT_STRINGS: &[&str] = &[
    "abi-gate",
    "abi-runtime-gate",
    "pgx-oauth-gate",
    "pggomtm-abi-allocator",
    "pggomtm-abi-error",
    "pggomtm-abi-panic",
    "pggomtm_abi_runtime_probe",
    "pggomtm_config_gate",
    "pggomtm_pgx_gate",
    "candidate-es256-pgx-gate",
    "HhhTL9R1TALzBB2cdc6zO4P_2BrHzk_ogsyxyYvFiW4",
    "pGwxHE4v9A3ZajZT5uRURdMt_khuztdcepDGoYiBwKM",
    "usr_pgx_gate",
    "cli_pgx_gate",
    "dlg_pgx_gate",
];
const PRODUCTION_FEATURE_IDENTITY: &str = r#""features":["pg18"]"#;
const RAW_TEST_SIGNING_KEY: [u8; 32] = [7; 32];

fn dependency_violations(tree: &str) -> Vec<String> {
    let mut violations = BTreeSet::new();
    for line in tree.lines() {
        let Some(package) = line.split_whitespace().next() else {
            continue;
        };
        if FORBIDDEN_NORMAL_DEPENDENCIES.contains(&package) {
            violations.insert(format!("forbidden normal dependency {package}"));
        }
    }
    violations.into_iter().collect()
}

fn source_violations(sources: &[(PathBuf, String)]) -> Vec<String> {
    let mut violations = BTreeSet::new();
    for (path, source) in sources {
        for fragment in FORBIDDEN_SOURCE_FRAGMENTS {
            if source.contains(fragment) {
                violations.insert(format!(
                    "{} contains forbidden production fragment {fragment}",
                    path.display()
                ));
            }
        }
    }
    violations.into_iter().collect()
}

fn dynamic_dependency_violations(readelf: &str) -> Vec<String> {
    let mut violations = BTreeSet::new();
    let mut dependencies = BTreeSet::new();
    for line in readelf.lines() {
        let Some(start) = line.find("Shared library: [") else {
            continue;
        };
        let dependency = &line[start + "Shared library: [".len()..];
        let Some(end) = dependency.find(']') else {
            violations.insert("malformed DT_NEEDED entry".to_owned());
            continue;
        };
        let dependency = &dependency[..end];
        dependencies.insert(dependency);
        if !ALLOWED_DYNAMIC_DEPENDENCIES.contains(&dependency) {
            violations.insert(format!("forbidden dynamic dependency {dependency}"));
        }
    }
    if dependencies.is_empty() {
        violations.insert("no DT_NEEDED entries were parsed".to_owned());
    }
    violations.into_iter().collect()
}

fn undefined_symbol_violations(nm: &str) -> Vec<String> {
    let mut violations = BTreeSet::new();
    for line in nm.lines() {
        let Some(versioned_symbol) = line.split_whitespace().last() else {
            continue;
        };
        let symbol = versioned_symbol
            .split('@')
            .next()
            .unwrap_or(versioned_symbol);
        if FORBIDDEN_UNDEFINED_SYMBOLS.contains(&symbol)
            || FORBIDDEN_UNDEFINED_SYMBOL_PREFIXES
                .iter()
                .any(|prefix| symbol.starts_with(prefix))
        {
            violations.insert(format!("forbidden undefined symbol {symbol}"));
        }
    }
    violations.into_iter().collect()
}

fn defined_symbol_violations(nm: &str) -> Vec<String> {
    let defined = nm
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .collect::<BTreeSet<_>>();
    let allowed = ALLOWED_DEFINED_SYMBOLS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut violations = BTreeSet::new();

    for symbol in defined.difference(&allowed) {
        violations.insert(format!("unexpected exported symbol {symbol}"));
    }
    for symbol in allowed.difference(&defined) {
        violations.insert(format!("missing required exported symbol {symbol}"));
    }
    violations.into_iter().collect()
}

fn elf_header_violations(header: &str) -> Vec<String> {
    [
        ("Class:                             ELF64", "ELF64 class"),
        (
            "Data:                              2's complement, little endian",
            "little-endian data",
        ),
        (
            "Type:                              DYN (Shared object file)",
            "shared-object type",
        ),
        (
            "Machine:                           Advanced Micro Devices X86-64",
            "amd64 machine",
        ),
    ]
    .into_iter()
    .filter(|(fragment, _)| !header.contains(fragment))
    .map(|(_, description)| format!("missing required ELF {description}"))
    .collect()
}

fn artifact_string_violations(strings: &str) -> Vec<String> {
    let normalized = strings.to_ascii_lowercase();
    FORBIDDEN_ARTIFACT_STRINGS
        .iter()
        .filter(|fragment| normalized.contains(**fragment))
        .map(|fragment| format!("forbidden artifact string {fragment}"))
        .collect()
}

fn production_artifact_violations(bytes: &[u8], strings: &str) -> Vec<String> {
    let mut violations = BTreeSet::new();
    for fragment in FORBIDDEN_GATE_ARTIFACT_STRINGS {
        if strings.contains(fragment) {
            violations.insert(format!("forbidden gate artifact string {fragment}"));
        }
    }
    if !strings.contains(PRODUCTION_FEATURE_IDENTITY) {
        violations.insert("missing exact production feature identity".to_owned());
    }
    if bytes
        .windows(RAW_TEST_SIGNING_KEY.len())
        .any(|window| window == RAW_TEST_SIGNING_KEY)
    {
        violations.insert("embedded raw test signing key".to_owned());
    }
    if contains_compact_jwt(bytes) {
        violations.insert("embedded compact JWT".to_owned());
    }
    violations.into_iter().collect()
}

fn contains_compact_jwt(bytes: &[u8]) -> bool {
    for start in 0..bytes.len().saturating_sub(3) {
        if bytes.get(start..start + 3) != Some(b"eyJ") {
            continue;
        }

        let mut segment_lengths = [0_usize; 3];
        let mut segment = 0_usize;
        for byte in &bytes[start..] {
            if *byte == b'.' {
                if segment >= 2 {
                    break;
                }
                segment += 1;
            } else if byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-') {
                segment_lengths[segment] += 1;
            } else {
                break;
            }
        }
        if segment == 2 && segment_lengths.into_iter().all(|length| length >= 8) {
            return true;
        }
    }
    false
}

#[test]
fn policy_rejects_online_and_database_dependency_fixtures() {
    let violations = dependency_violations(
        "pggomtm v0.1.0\nreqwest v0.12.0\nhickory-resolver v0.25.0\nsqlx-core v0.8.0\n",
    );

    assert!(violations.iter().any(|value| value.contains("reqwest")));
    assert!(
        violations
            .iter()
            .any(|value| value.contains("hickory-resolver"))
    );
    assert!(violations.iter().any(|value| value.contains("sqlx-core")));
}

#[test]
fn policy_rejects_forbidden_source_capability_fixtures() {
    let sources = vec![
        (
            PathBuf::from("src/network.rs"),
            "use std::net::TcpStream;".to_owned(),
        ),
        (
            PathBuf::from("src/sql.rs"),
            "pgrx::pg_sys::SPI_connect();".to_owned(),
        ),
        (
            PathBuf::from("src/config.rs"),
            "let fallback_issuer = private_key_path;".to_owned(),
        ),
    ];
    let violations = source_violations(&sources);

    assert!(violations.iter().any(|value| value.contains("std::net")));
    assert!(violations.iter().any(|value| value.contains("SPI_")));
    assert!(
        violations
            .iter()
            .any(|value| value.contains("fallback_issuer"))
    );
    assert!(
        violations
            .iter()
            .any(|value| value.contains("private_key_path"))
    );
}

#[test]
fn policy_rejects_forbidden_elf_capability_fixtures() {
    let dynamic = dynamic_dependency_violations(
        "Shared library: [libc.so.6]\nShared library: [libcurl.so.4]\n",
    );
    let symbols =
        undefined_symbol_violations(" U getaddrinfo@GLIBC_2.34\n U SPI_execute\n U PQconnectdb\n");
    let strings = artifact_string_violations(
        "fallback_issuer\nintrospection_endpoint\n-----BEGIN PRIVATE KEY-----\n",
    );

    assert!(dynamic.iter().any(|value| value.contains("libcurl.so.4")));
    assert!(symbols.iter().any(|value| value.contains("getaddrinfo")));
    assert!(symbols.iter().any(|value| value.contains("SPI_execute")));
    assert!(symbols.iter().any(|value| value.contains("PQconnectdb")));
    assert!(
        strings
            .iter()
            .any(|value| value.contains("fallback_issuer"))
    );
    assert!(
        strings
            .iter()
            .any(|value| value.contains("introspection_endpoint"))
    );
    assert!(
        strings
            .iter()
            .any(|value| value.contains("begin private key"))
    );
}

#[test]
fn policy_rejects_gate_symbols_keys_and_tokens() {
    let defined = defined_symbol_violations(
        "00000000 T Pg_magic_func\n00000010 T _PG_oauth_validator_module_init\n00000020 T pggomtm_test_probe\n",
    );
    assert!(
        defined
            .iter()
            .any(|value| value.contains("pggomtm_test_probe"))
    );

    let mut fixture = RAW_TEST_SIGNING_KEY.to_vec();
    fixture.extend_from_slice(b"eyJhbGciOiJFUzI1NiJ9.cGF5bG9hZDEyMzQ1.sigvalue1234");
    let violations =
        production_artifact_violations(&fixture, "candidate-es256-pgx-gate\nabi-runtime-gate\n");
    assert!(
        violations
            .iter()
            .any(|value| value.contains("test signing key"))
    );
    assert!(violations.iter().any(|value| value.contains("compact JWT")));
    assert!(
        violations
            .iter()
            .any(|value| value.contains("candidate-es256-pgx-gate"))
    );
    assert!(
        violations
            .iter()
            .any(|value| value.contains("abi-runtime-gate"))
    );
}

#[test]
fn policy_accepts_the_minimal_offline_fixture() {
    assert!(dependency_violations("pggomtm v0.1.0\npgrx v0.19.1\np256 v0.13.2\n").is_empty());
    assert!(
        source_violations(&[(
            PathBuf::from("src/runtime_config.rs"),
            "File::open(PUBLIC_JWKS_PATH)".to_owned(),
        )])
        .is_empty()
    );
    assert!(
        dynamic_dependency_violations(
            "Shared library: [libgcc_s.so.1]\nShared library: [libc.so.6]\nShared library: [ld-linux-x86-64.so.2]\n"
        )
        .is_empty()
    );
    assert!(undefined_symbol_violations(" U open64@GLIBC_2.2.5\n U read@GLIBC_2.2.5\n").is_empty());
    assert!(artifact_string_violations("issuer\naudience\n/etc/pggomtm/jwks.json\n").is_empty());
    assert!(
        defined_symbol_violations(
            "00000000 T Pg_magic_func\n00000010 T _PG_oauth_validator_module_init\n"
        )
        .is_empty()
    );
    assert!(
        elf_header_violations(
            "Class:                             ELF64\nData:                              2's complement, little endian\nType:                              DYN (Shared object file)\nMachine:                           Advanced Micro Devices X86-64\n"
        )
        .is_empty()
    );
    assert!(
        production_artifact_violations(
            b"production-module-without-fixtures",
            PRODUCTION_FEATURE_IDENTITY,
        )
        .is_empty()
    );
}

#[test]
#[ignore = "Docker production gate显式提供normal dependency tree与源码根目录"]
fn production_static_capability_boundary_is_closed() {
    let dependency_tree = read_required_file("PGGOMTM_NORMAL_DEPENDENCY_TREE");
    let source_root = required_path("PGGOMTM_PRODUCTION_SOURCE_ROOT");
    let mut sources = Vec::new();
    collect_production_sources(&source_root, &mut sources);

    let mut violations = dependency_violations(&dependency_tree);
    violations.extend(source_violations(&sources));
    assert_no_violations("production static capability boundary", &violations);
}

#[test]
#[ignore = "Docker production gate显式提供release ELF路径"]
fn production_elf_capability_boundary_is_closed() {
    let artifact = required_path("PGGOMTM_PRODUCTION_ARTIFACT");
    assert!(
        artifact.is_file(),
        "production artifact path must be a file"
    );

    let readelf = run_tool("readelf", &["--dynamic", "--wide"], &artifact);
    let header = run_tool("readelf", &["--file-header", "--wide"], &artifact);
    let undefined = run_tool("nm", &["--dynamic", "--undefined-only"], &artifact);
    let defined = run_tool("nm", &["--dynamic", "--defined-only"], &artifact);
    let strings = run_tool("strings", &["--all"], &artifact);
    let bytes = fs::read(&artifact)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", artifact.display()));
    let mut violations = dynamic_dependency_violations(&readelf);
    violations.extend(elf_header_violations(&header));
    violations.extend(undefined_symbol_violations(&undefined));
    violations.extend(defined_symbol_violations(&defined));
    violations.extend(artifact_string_violations(&strings));
    violations.extend(production_artifact_violations(&bytes, &strings));
    assert_no_violations("production ELF capability boundary", &violations);
}

fn required_path(variable: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{variable} must be set by the Docker production gate"))
}

fn read_required_file(variable: &str) -> String {
    let path = required_path(variable);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn collect_production_sources(root: &Path, sources: &mut Vec<(PathBuf, String)>) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("failed to enumerate {}: {error}", root.display()));
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("failed to inspect {}: {error}", path.display()));
        if file_type.is_dir() {
            if entry.file_name() != "tests" {
                collect_production_sources(&path, sources);
            }
        } else if file_type.is_file()
            && path.extension().is_some_and(|extension| extension == "rs")
            && entry.file_name() != "tests.rs"
        {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            sources.push((path, source));
        } else if file_type.is_symlink() {
            panic!(
                "production source entry must not be a symlink: {}",
                path.display()
            );
        }
    }
}

fn run_tool(program: &str, arguments: &[&str], artifact: &Path) -> String {
    let output = Command::new(program)
        .args(arguments)
        .arg(artifact)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute {program}: {error}"));
    assert!(
        output.status.success(),
        "{program} rejected the production artifact"
    );
    String::from_utf8(output.stdout)
        .unwrap_or_else(|_| panic!("{program} emitted non-UTF-8 output"))
}

fn assert_no_violations(boundary: &str, violations: &[String]) {
    assert!(
        violations.is_empty(),
        "{boundary} rejected: {}",
        violations.join(", ")
    );
}
