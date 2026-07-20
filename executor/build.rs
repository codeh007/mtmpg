use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

const POSTGRES_MAJOR: u32 = 18;
const BINDINGS_FILE: &str = "mtmpg_executor_libpq_bindings.rs";
const LIBPQ_FUNCTIONS: &str = concat!(
    "^(PQcancelBlocking|PQcancelCreate|PQcancelFinish|PQclear|PQcmdStatus|PQcmdTuples|",
    "PQconnectPoll|PQconnectStartParams|PQconsumeInput|PQerrorMessage|PQfinish|PQflush|",
    "PQfname|PQftype|PQgetCurrentTimeUSec|PQgetResult|PQgetisnull|PQgetlength|PQgetvalue|",
    "PQisBusy|PQlibVersion|PQnfields|PQntuples|PQresultErrorField|PQresultStatus|",
    "PQsendQueryParams|PQsetAuthDataHook|PQsetErrorContextVisibility|PQsetErrorVerbosity|",
    "PQsetnonblocking|PQsetNoticeProcessor|PQsocket|PQsocketPoll|PQtransactionStatus)$"
);
const LIBPQ_TYPES: &str = concat!(
    "^(ConnStatusType|ExecStatusType|Oid|PGauthData|PGcancelConn|PGconn|PGContextVisibility|",
    "PGoauthBearerRequest|PGresult|PGTransactionStatusType|PGVerbosity|PostgresPollingStatusType|",
    "PQauthDataHook_type|PQnoticeProcessor|pg_cancel_conn|pg_conn|pg_result|pg_usec_time_t)$"
);

type BuildResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    println!("cargo:rerun-if-env-changed=PGRX_PG_CONFIG_PATH");
    if let Err(error) = generate_bindings() {
        panic!("failed to generate PostgreSQL libpq bindings: {error}");
    }
}

fn generate_bindings() -> BuildResult<()> {
    let pg_config = required_path("PGRX_PG_CONFIG_PATH")?;
    let version = pg_config_line(&pg_config, "--version")?;
    let major = version
        .strip_prefix("PostgreSQL ")
        .and_then(|value| value.split('.').next())
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or("pg_config returned an unsupported version string")?;
    if major != POSTGRES_MAJOR {
        return Err(format!("PostgreSQL major {major} is unsupported").into());
    }

    let include_dir = PathBuf::from(pg_config_line(&pg_config, "--includedir")?);
    if !include_dir.is_absolute() || !include_dir.is_dir() {
        return Err("pg_config client include directory is unavailable".into());
    }
    let header = required_file(&include_dir.join("libpq-fe.h"))?;
    let include_dir = utf8_path(&include_dir)?;
    let header = utf8_path(&header)?;
    println!("cargo:rerun-if-changed={header}");

    pkg_config::Config::new()
        .atleast_version("18")
        .probe("libpq")?;
    let bindings = bindgen::Builder::default()
        .header(header)
        .detect_include_paths(false)
        .clang_arg(format!("-I{include_dir}"))
        .allowlist_function(LIBPQ_FUNCTIONS)
        .allowlist_type(LIBPQ_TYPES)
        .allowlist_var(concat!(
            "^(CONNECTION_|PGRES_|PG_DIAG_SQLSTATE$|PQAUTHDATA_|PQERRORS_|",
            "PQSHOW_CONTEXT_|PQTRANS_).*$"
        ))
        .allowlist_recursively(false)
        .generate_comments(false)
        .layout_tests(false)
        .formatter(bindgen::Formatter::None)
        .generate()?;

    let output = required_path("OUT_DIR")?.join(BINDINGS_FILE);
    bindings.write_to_file(output)?;
    Ok(())
}

fn required_path(name: &str) -> BuildResult<PathBuf> {
    let path = PathBuf::from(env::var_os(name).ok_or_else(|| format!("{name} is required"))?);
    if !path.is_absolute() {
        return Err(format!("{name} must be absolute").into());
    }
    Ok(path)
}

fn required_file(path: &Path) -> BuildResult<PathBuf> {
    if !path.is_file() {
        return Err(format!(
            "required PostgreSQL header is unavailable: {}",
            path.display()
        )
        .into());
    }
    Ok(path.to_path_buf())
}

fn utf8_path(path: &Path) -> BuildResult<&str> {
    path.to_str()
        .ok_or_else(|| format!("path must be UTF-8: {}", path.display()).into())
}

fn pg_config_line(pg_config: &Path, argument: &str) -> BuildResult<String> {
    let output = Command::new(pg_config).arg(argument).output()?;
    if !output.status.success() {
        return Err(format!("pg_config {argument} failed").into());
    }
    let value = String::from_utf8(output.stdout)?.trim().to_owned();
    if value.is_empty() {
        return Err(format!("pg_config {argument} returned no value").into());
    }
    Ok(value)
}
