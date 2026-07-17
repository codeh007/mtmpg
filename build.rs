use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const APPROVED_POSTGRESQL_VERSION: &str = "PostgreSQL 18.4";
const APPROVED_POSTGRESQL_SOURCE_SHA256: &str =
    "81a81ec695fb0c7901407defaa1d2f7973617154cf27ba74e3a7ab8e64436094";
const APPROVED_OAUTH_HEADER_SHA256: &str =
    "be015ae68deef28a906c8739bc653ca90a4c6966c10f0efd3bd926efb4958bcf";
const APPROVED_OAUTH_BINDINGS_SHA256: &str =
    "b6f8bf810c467f74a0e43f9019f00cfd517cc881c9b606175818ca1b17204beb";
const APPROVED_RUNTIME_BASE: &str = "postgres:18.4-bookworm";
const APPROVED_RUNTIME_BASE_SHA256: &str =
    "1961f96e6029a02c3812d7cb329a3b03a3ac2bb067058dec17b0f5596aca9296";
const APPROVED_RUST_VERSION: &str = "1.97.1";
const APPROVED_PGRX_VERSION: &str = "0.19.1";
const APPROVED_JOSE_VERSION: &str = "1.0.4";
const APPROVED_TARGET: &str = "x86_64-unknown-linux-gnu";
const BUILD_IDENTITY_FILE: &str = "pggomtm_build_identity.json";
const BINDINGS_FILE: &str = "pggomtm_oauth_bindings.rs";
const CALLBACK_ABI_PATTERN: &str =
    "^(ValidatorStartupCB|ValidatorShutdownCB|ValidatorValidateCB|OAuthValidatorModuleInit)$";
const EXPECTED_PUBLIC_ITEMS: [&str; 8] = [
    "OAuthValidatorCallbacks",
    "OAuthValidatorModuleInit",
    "PG_OAUTH_VALIDATOR_MAGIC",
    "ValidatorModuleResult",
    "ValidatorModuleState",
    "ValidatorShutdownCB",
    "ValidatorStartupCB",
    "ValidatorValidateCB",
];
const CALLBACK_TYPES: [&str; 4] = [
    "ValidatorStartupCB",
    "ValidatorShutdownCB",
    "ValidatorValidateCB",
    "OAuthValidatorModuleInit",
];
const EXPECTED_COMPACT_ABI_FRAGMENTS: [&str; 7] = [
    "pubstructValidatorModuleState{pubsversion:::std::os::raw::c_int,pubprivate_data:*mut::std::os::raw::c_void,}",
    "pubstructValidatorModuleResult{pubauthorized:bool,pubauthn_id:*mut::std::os::raw::c_char,}",
    "pubtypeValidatorStartupCB=::std::option::Option<unsafeextern\"C-unwind\"fn(state:*mutValidatorModuleState)>;",
    "pubtypeValidatorShutdownCB=::std::option::Option<unsafeextern\"C-unwind\"fn(state:*mutValidatorModuleState)>;",
    "pubtypeValidatorValidateCB=::std::option::Option<unsafeextern\"C-unwind\"fn(state:*constValidatorModuleState,token:*const::std::os::raw::c_char,role:*const::std::os::raw::c_char,result:*mutValidatorModuleResult)->bool>;",
    "pubstructOAuthValidatorCallbacks{pubmagic:uint32,pubstartup_cb:ValidatorStartupCB,pubshutdown_cb:ValidatorShutdownCB,pubvalidate_cb:ValidatorValidateCB,}",
    "pubtypeOAuthValidatorModuleInit=::std::option::Option<unsafeextern\"C-unwind\"fn()->*constOAuthValidatorCallbacks>;",
];
const AMBIENT_CLANG_ENV: [&str; 11] = [
    "CCC_OVERRIDE_OPTIONS",
    "CLANG_PATH",
    "COMPILER_PATH",
    "CPATH",
    "CPLUS_INCLUDE_PATH",
    "C_INCLUDE_PATH",
    "LIBCLANG_PATH",
    "LIBCLANG_STATIC_PATH",
    "LLVM_CONFIG_PATH",
    "OBJCPLUS_INCLUDE_PATH",
    "OBJC_INCLUDE_PATH",
];
const FEATURE_ENV: [(&str, &str); 4] = [
    ("CARGO_FEATURE_PG18", "pg18"),
    ("CARGO_FEATURE_ABI_GATE", "abi-gate"),
    ("CARGO_FEATURE_ABI_RUNTIME_GATE", "abi-runtime-gate"),
    ("CARGO_FEATURE_PGX_OAUTH_GATE", "pgx-oauth-gate"),
];

type BuildResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    println!("cargo:rerun-if-env-changed=PGRX_PG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=TARGET");
    for (variable, _) in FEATURE_ENV {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    if let Err(error) = controlled_generate_bindings() {
        panic!("failed to generate the approved PostgreSQL OAuth ABI bindings: {error}");
    }
}

fn controlled_generate_bindings() -> BuildResult<()> {
    let target = build_target()?;
    let features = build_features()?;
    let bindgen_env = bindgen_extra_clang_args_names(&target);
    for variable in bindgen_env
        .iter()
        .map(String::as_str)
        .chain(AMBIENT_CLANG_ENV)
    {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    reject_ambient_clang_configuration(&bindgen_env)?;
    generate_bindings(&target, &features)
}

fn generate_bindings(target: &str, features: &[&str]) -> BuildResult<()> {
    let pg_config = target_pg_config()?;
    println!(
        "cargo:rerun-if-changed={}",
        approved_utf8_path(
            &pg_config,
            "canonical pg_config path must be valid UTF-8 and safe for Cargo metadata",
        )?
    );

    let version = pg_config_line(&pg_config, "--version")?;
    if version != APPROVED_POSTGRESQL_VERSION {
        return fail("PGRX_PG_CONFIG_PATH does not identify approved PostgreSQL 18.4");
    }

    let include_dir = server_include_dir(&pg_config)?;
    let postgres_header = approved_regular_file(&include_dir, "postgres.h")?;
    let oauth_header = approved_regular_file(&include_dir, "libpq/oauth.h")?;

    let include_dir = approved_utf8_path(
        &include_dir,
        "canonical server include path must be safe for Clang and valid UTF-8",
    )?;
    let postgres_header = approved_utf8_path(
        &postgres_header,
        "canonical PostgreSQL header path must be safe for Clang and valid UTF-8",
    )?;
    let oauth_header = approved_utf8_path(
        &oauth_header,
        "canonical OAuth header path must be safe for Clang and valid UTF-8",
    )?;

    println!("cargo:rerun-if-changed={include_dir}");
    println!("cargo:rerun-if-changed={postgres_header}");
    println!("cargo:rerun-if-changed={oauth_header}");

    let header = fs::read(oauth_header)
        .map_err(|_| build_error("approved OAuth header could not be read"))?;
    let header_sha256 = format!("{:x}", Sha256::digest(&header));
    if header_sha256 != APPROVED_OAUTH_HEADER_SHA256 {
        return fail("OAuth header SHA-256 is not approved for this build variant");
    }

    let bindings = bindgen::Builder::default()
        .header(postgres_header)
        .header(oauth_header)
        .detect_include_paths(false)
        .clang_arg(format!("-I{include_dir}"))
        .allowlist_var("^PG_OAUTH_VALIDATOR_MAGIC$")
        .allowlist_type(
            "^(ValidatorModuleState|ValidatorModuleResult|ValidatorStartupCB|ValidatorShutdownCB|ValidatorValidateCB|OAuthValidatorCallbacks|OAuthValidatorModuleInit)$",
        )
        .allowlist_recursively(false)
        .generate_comments(false)
        .layout_tests(false)
        .formatter(bindgen::Formatter::None)
        .override_abi(bindgen::Abi::CUnwind, CALLBACK_ABI_PATTERN)
        .generate()
        .map_err(|_| build_error("bindgen rejected the approved PostgreSQL OAuth headers"))?;

    let generated = bindings.to_string();
    validate_generated_bindings(&generated)?;
    let bindings_sha256 = format!("{:x}", Sha256::digest(generated.as_bytes()));
    if bindings_sha256 != APPROVED_OAUTH_BINDINGS_SHA256 {
        return fail(format!(
            "generated OAuth bindings SHA-256 is not approved: {bindings_sha256}"
        ));
    }

    let out_dir = output_dir()?;
    let bindings_path = out_dir.join(BINDINGS_FILE);
    fs::write(&bindings_path, generated.as_bytes())
        .map_err(|_| build_error("generated OAuth bindings could not be written to OUT_DIR"))?;
    let written = fs::read(&bindings_path)
        .map_err(|_| build_error("final OAuth bindings could not be read from OUT_DIR"))?;
    if written != generated.as_bytes() {
        return fail("final OUT_DIR bindings differ from the validated materialized bytes");
    }
    let written_sha256 = format!("{:x}", Sha256::digest(&written));
    if written_sha256 != bindings_sha256 {
        return fail("final OUT_DIR bindings digest differs from the validated bindings digest");
    }

    let build_identity =
        canonical_build_identity(target, features, &header_sha256, &bindings_sha256);
    let build_identity_path = out_dir.join(BUILD_IDENTITY_FILE);
    fs::write(&build_identity_path, build_identity.as_bytes())
        .map_err(|_| build_error("artifact identity could not be written to OUT_DIR"))?;
    let written_identity = fs::read(&build_identity_path)
        .map_err(|_| build_error("artifact identity could not be read from OUT_DIR"))?;
    if written_identity != build_identity.as_bytes() {
        return fail("final OUT_DIR artifact identity differs from its canonical bytes");
    }
    let build_identity_sha256 = format!("{:x}", Sha256::digest(&written_identity));

    println!("cargo:rustc-env=PG_OAUTH_HEADER_SHA256={header_sha256}");
    println!("cargo:rustc-env=PG_OAUTH_BINDINGS_SHA256={bindings_sha256}");
    println!("cargo:rustc-env=PGGOMTM_BUILD_IDENTITY_JSON={build_identity}");
    println!("cargo:rustc-env=PGGOMTM_BUILD_IDENTITY_SHA256={build_identity_sha256}");
    Ok(())
}

fn target_pg_config() -> BuildResult<PathBuf> {
    let configured = env::var("PGRX_PG_CONFIG_PATH").map_err(|_| {
        build_error("PGRX_PG_CONFIG_PATH must be set to one absolute UTF-8 pg_config path")
    })?;
    if configured.is_empty() || configured.trim() != configured {
        return fail("PGRX_PG_CONFIG_PATH is empty or contains surrounding whitespace");
    }

    let configured = PathBuf::from(configured);
    if !configured.is_absolute() || configured.file_name() != Some(OsStr::new("pg_config")) {
        return fail("PGRX_PG_CONFIG_PATH must name one absolute pg_config executable");
    }

    let canonical = fs::canonicalize(&configured)
        .map_err(|_| build_error("PGRX_PG_CONFIG_PATH could not be resolved"))?;
    approved_utf8_path(
        &canonical,
        "canonical pg_config path must be valid UTF-8 and safe for Cargo metadata",
    )?;
    if canonical != configured {
        return fail("PGRX_PG_CONFIG_PATH must already be canonical");
    }
    if !fs::metadata(&canonical)
        .map_err(|_| build_error("PGRX_PG_CONFIG_PATH metadata could not be read"))?
        .is_file()
    {
        return fail("PGRX_PG_CONFIG_PATH does not resolve to a regular file");
    }

    Ok(canonical)
}

fn server_include_dir(pg_config: &Path) -> BuildResult<PathBuf> {
    let reported = pg_config_line(pg_config, "--includedir-server")?;
    let reported = PathBuf::from(reported);
    if !reported.is_absolute() {
        return fail("pg_config --includedir-server did not return an absolute path");
    }

    let canonical = fs::canonicalize(&reported)
        .map_err(|_| build_error("server include directory could not be resolved"))?;
    approved_utf8_path(
        &canonical,
        "canonical server include path must be safe for Clang and valid UTF-8",
    )?;
    if canonical != reported {
        return fail("server include directory must already be canonical");
    }
    if !fs::metadata(&canonical)
        .map_err(|_| build_error("server include directory metadata could not be read"))?
        .is_dir()
    {
        return fail("pg_config --includedir-server did not resolve to a directory");
    }

    Ok(canonical)
}

fn approved_regular_file(include_dir: &Path, relative: &str) -> BuildResult<PathBuf> {
    let expected = include_dir.join(relative);
    let canonical = fs::canonicalize(&expected)
        .map_err(|_| build_error("required PostgreSQL server header is missing"))?;
    if canonical != expected {
        return fail(
            "required PostgreSQL server header must not escape its canonical include path",
        );
    }
    if !fs::metadata(&canonical)
        .map_err(|_| build_error("required PostgreSQL server header metadata could not be read"))?
        .is_file()
    {
        return fail("required PostgreSQL server header is not a regular file");
    }

    Ok(canonical)
}

fn pg_config_line(pg_config: &Path, argument: &str) -> BuildResult<String> {
    let output = Command::new(pg_config)
        .arg(argument)
        .output()
        .map_err(|_| build_error("target pg_config could not be executed"))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return fail("target pg_config returned an unsuccessful or noisy result");
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| build_error("target pg_config output must be valid UTF-8"))?;
    let line = stdout.strip_suffix('\n').unwrap_or(&stdout);
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.is_empty() || line.contains('\r') || line.contains('\n') || line.trim() != line {
        return fail("target pg_config must return exactly one non-empty output line");
    }

    Ok(line.to_owned())
}

fn output_dir() -> BuildResult<PathBuf> {
    let out_dir = env::var("OUT_DIR")
        .map_err(|_| build_error("Cargo OUT_DIR must be present and valid UTF-8"))?;
    let out_dir = PathBuf::from(out_dir);
    if !out_dir.is_absolute() || !out_dir.is_dir() {
        return fail("Cargo OUT_DIR must be an existing absolute directory");
    }
    Ok(out_dir)
}

fn build_target() -> BuildResult<String> {
    let target = env::var("TARGET")
        .map_err(|_| build_error("Cargo TARGET must be present and valid UTF-8"))?;
    if target.is_empty()
        || !target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return fail("Cargo TARGET must be one non-empty target identifier");
    }
    if target != APPROVED_TARGET {
        return fail("Cargo TARGET is not approved for this build variant");
    }
    Ok(target)
}

fn build_features() -> BuildResult<Vec<&'static str>> {
    let mut features = Vec::new();
    for (variable, feature) in FEATURE_ENV {
        if env::var_os(variable).is_some() {
            features.push(feature);
        }
    }
    if !features.contains(&"pg18") {
        return fail("the approved build variant requires the pg18 Cargo feature");
    }

    for (variable, _) in env::vars_os() {
        let Some(variable) = variable.to_str() else {
            continue;
        };
        if variable.starts_with("CARGO_FEATURE_")
            && variable != "CARGO_FEATURE_DEFAULT"
            && !FEATURE_ENV.iter().any(|(allowed, _)| variable == *allowed)
        {
            return fail(format!(
                "Cargo feature {variable} is not approved for an artifact build variant"
            ));
        }
    }

    Ok(features)
}

fn canonical_build_identity(
    target: &str,
    features: &[&str],
    header_sha256: &str,
    bindings_sha256: &str,
) -> String {
    let features = features
        .iter()
        .map(|feature| format!("\"{feature}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"schema\":\"pggomtm-build-identity/v1\",",
            "\"module_version\":\"{}\",",
            "\"features\":[{}],",
            "\"rust\":{{\"version\":\"{}\",\"target\":\"{}\"}},",
            "\"dependencies\":{{\"pgrx\":\"{}\",\"jose_implementation\":\"jaws\",\"jose_version\":\"{}\"}},",
            "\"postgresql\":{{\"source_version\":\"18.4\",\"pg_version_num\":180004,",
            "\"source_sha256\":\"{}\",\"oauth_header_sha256\":\"{}\",",
            "\"oauth_bindings_sha256\":\"{}\",\"runtime_base\":\"{}\",",
            "\"runtime_base_sha256\":\"{}\"}},",
            "\"platform\":{{\"os\":\"linux\",\"arch\":\"amd64\",\"libc\":\"glibc\"}}}}"
        ),
        env!("CARGO_PKG_VERSION"),
        features,
        APPROVED_RUST_VERSION,
        target,
        APPROVED_PGRX_VERSION,
        APPROVED_JOSE_VERSION,
        APPROVED_POSTGRESQL_SOURCE_SHA256,
        header_sha256,
        bindings_sha256,
        APPROVED_RUNTIME_BASE,
        APPROVED_RUNTIME_BASE_SHA256,
    )
}

fn bindgen_extra_clang_args_names(target: &str) -> [String; 3] {
    [
        "BINDGEN_EXTRA_CLANG_ARGS".to_owned(),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{target}"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{}", target.replace('-', "_")),
    ]
}

fn reject_ambient_clang_configuration(bindgen_env: &[String; 3]) -> BuildResult<()> {
    for variable in bindgen_env
        .iter()
        .map(String::as_str)
        .chain(AMBIENT_CLANG_ENV)
    {
        if env::var_os(variable).is_some() {
            return fail("ambient bindgen or Clang configuration is not permitted");
        }
    }
    Ok(())
}

fn approved_utf8_path<'a>(path: &'a Path, error: &str) -> BuildResult<&'a str> {
    let path = path.to_str().ok_or_else(|| build_error(error))?;
    if path
        .chars()
        .any(|character| character.is_control() || matches!(character, '"' | '\\'))
    {
        return fail(error);
    }
    Ok(path)
}

fn validate_generated_bindings(generated: &str) -> BuildResult<()> {
    let compact = generated
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let expected = EXPECTED_PUBLIC_ITEMS.into_iter().collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();

    for prefix in ["pubconst", "pubstruct", "pubtype"] {
        let mut remainder = compact.as_str();
        while let Some(start) = remainder.find(prefix) {
            remainder = &remainder[start + prefix.len()..];
            let item = public_item_name(remainder);
            if !actual.insert(item) {
                return fail("bindgen emitted a duplicate public OAuth ABI item");
            }
        }
    }

    if actual != expected {
        return fail("bindgen output does not exactly match the approved OAuth ABI allowlist");
    }
    if compact.contains("_PG_oauth_validator_module_init") {
        return fail(
            "bindgen emitted the runtime init symbol instead of only its approved typedef",
        );
    }
    if compact.contains("pubfn") || compact.contains("pubstatic") {
        return fail("bindgen emitted an unapproved OAuth function or static symbol");
    }
    if !compact.contains("pubconstPG_OAUTH_VALIDATOR_MAGIC:u32=539296288;") {
        return fail("bindgen did not preserve the approved PostgreSQL OAuth magic value");
    }
    if !compact.contains("pubmagic:uint32,") {
        return fail("bindgen did not preserve PostgreSQL's uint32 callback magic field");
    }
    for fragment in EXPECTED_COMPACT_ABI_FRAGMENTS {
        if !compact.contains(fragment) {
            return fail("bindgen output does not match the approved OAuth ABI field signatures");
        }
    }

    for callback in CALLBACK_TYPES {
        let marker = format!("pubtype{callback}");
        let definition = compact
            .split_once(&marker)
            .and_then(|(_, remainder)| remainder.split_once(';'))
            .map(|(definition, _)| definition)
            .ok_or_else(|| build_error("bindgen omitted an approved callback typedef"))?;
        if !definition.contains("extern\"C-unwind\"fn") {
            return fail("bindgen callback typedef is not using the required C-unwind ABI");
        }
    }

    Ok(())
}

fn public_item_name(remainder: &str) -> &str {
    let end = remainder
        .find([':', '=', '{', ';'])
        .unwrap_or(remainder.len());
    &remainder[..end]
}

fn fail<T>(message: impl Into<String>) -> BuildResult<T> {
    Err(build_error(message))
}

fn build_error(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::other(message.into()).into()
}

// PostgreSQL 18 OAuth ABI 与 validator 契约：
// https://www.postgresql.org/docs/18/oauth-validators.html
// https://github.com/postgres/postgres/blob/REL_18_4/src/include/libpq/oauth.h
// https://github.com/postgres/postgres/blob/REL_18_4/src/test/modules/oauth_validator/validator.c
// bindgen 构建期生成与闭集 allowlist：
// https://rust-lang.github.io/rust-bindgen/tutorial-3.html
// https://rust-lang.github.io/rust-bindgen/allowlisting.html
// bindgen 0.72.1 官方测试证明 typedef ABI override 与 C-unwind 支持：
// https://github.com/rust-lang/rust-bindgen/blob/v0.72.1/bindgen-tests/tests/headers/abi-override.h
// https://github.com/rust-lang/rust-bindgen/blob/v0.72.1/bindgen-tests/tests/expectations/tests/abi-override.rs
// https://github.com/rust-lang/rust-bindgen/blob/v0.72.1/bindgen-tests/tests/headers/c-unwind-abi-override.h
// https://github.com/rust-lang/rust-bindgen/blob/v0.72.1/bindgen-tests/tests/expectations/tests/c-unwind-abi-override.rs
