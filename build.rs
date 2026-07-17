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
const APPROVED_OAUTH_HEADER_SHA256: &str =
    "be015ae68deef28a906c8739bc653ca90a4c6966c10f0efd3bd926efb4958bcf";
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

type BuildResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    println!("cargo:rerun-if-env-changed=PGRX_PG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=TARGET");

    if let Err(error) = controlled_generate_bindings() {
        panic!("failed to generate the approved PostgreSQL OAuth ABI bindings: {error}");
    }
}

fn controlled_generate_bindings() -> BuildResult<()> {
    let target = build_target()?;
    let bindgen_env = bindgen_extra_clang_args_names(&target);
    for variable in bindgen_env
        .iter()
        .map(String::as_str)
        .chain(AMBIENT_CLANG_ENV)
    {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    reject_ambient_clang_configuration(&bindgen_env)?;
    generate_bindings()
}

fn generate_bindings() -> BuildResult<()> {
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
        .override_abi(bindgen::Abi::CUnwind, CALLBACK_ABI_PATTERN)
        .generate()
        .map_err(|_| build_error("bindgen rejected the approved PostgreSQL OAuth headers"))?;

    let generated = bindings.to_string();
    validate_generated_bindings(&generated)?;

    let out_dir = output_dir()?;
    bindings
        .write_to_file(out_dir.join(BINDINGS_FILE))
        .map_err(|_| build_error("generated OAuth bindings could not be written to OUT_DIR"))?;

    println!("cargo:rustc-env=PG_OAUTH_HEADER_SHA256={header_sha256}");
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
    Ok(target)
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
    let expected = EXPECTED_PUBLIC_ITEMS.into_iter().collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();

    for line in generated.lines() {
        let item = ["pub const ", "pub struct ", "pub type "]
            .into_iter()
            .find_map(|prefix| public_item_name(line, prefix));
        if let Some(item) = item {
            if !actual.insert(item) {
                return fail("bindgen emitted a duplicate public OAuth ABI item");
            }
        } else if line.starts_with("pub ") {
            return fail("bindgen expanded the public OAuth ABI outside the approved item kinds");
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub static ") {
            return fail("bindgen emitted an unapproved OAuth function or static symbol");
        }
    }

    if actual != expected {
        return fail("bindgen output does not exactly match the approved OAuth ABI allowlist");
    }
    if generated.contains("_PG_oauth_validator_module_init") {
        return fail(
            "bindgen emitted the runtime init symbol instead of only its approved typedef",
        );
    }
    if !generated.contains("pub magic: uint32,") {
        return fail("bindgen did not preserve PostgreSQL's uint32 callback magic field");
    }

    for callback in CALLBACK_TYPES {
        let marker = format!("pub type {callback}");
        let definition = generated
            .split_once(&marker)
            .and_then(|(_, remainder)| remainder.split_once(';'))
            .map(|(definition, _)| definition)
            .ok_or_else(|| build_error("bindgen omitted an approved callback typedef"))?;
        if !definition.contains("extern \"C-unwind\" fn") {
            return fail("bindgen callback typedef is not using the required C-unwind ABI");
        }
    }

    Ok(())
}

fn public_item_name<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let remainder = line.strip_prefix(prefix)?;
    let end = remainder
        .find([' ', ':', '=', '{', ';'])
        .unwrap_or(remainder.len());
    Some(&remainder[..end])
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
