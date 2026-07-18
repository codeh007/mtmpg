#![cfg(unix)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

const APPROVED_MAGIC: u32 = 0x2025_0220;
const ATTACK_MAGIC: u32 = 0x0bad_cafe;
const APPROVED_BINDINGS_SHA256: &str =
    "b6f8bf810c467f74a0e43f9019f00cfd517cc881c9b606175818ca1b17204beb";
const BUILD_IDENTITY_FILE: &str = "pggomtm_build_identity.json";
const BINDINGS_FILE: &str = "pggomtm_oauth_bindings.rs";
const TARGET: &str = "x86_64-unknown-linux-gnu";
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

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new() -> Self {
        let root = env::temp_dir().join(format!("pggomtm-oauth-provenance-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale provenance fixture");
        }
        fs::create_dir(&root).expect("create provenance fixture");
        Self { root }
    }

    fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
#[ignore = "Native CI 的 ABI 步骤显式运行这个真实 generator 负向门禁"]
fn real_generator_rejects_unapproved_provenance_inputs() {
    let fixture = TempTree::new();
    let build_script = current_build_script();
    let official_pg_config = canonical_env_path("PGRX_PG_CONFIG_PATH");
    let official_include = pg_config_include_dir(&official_pg_config);
    let official_header = fs::read_to_string(official_include.join("libpq/oauth.h"))
        .expect("read approved OAuth header");
    let attack_header = official_header.replacen("0x20250220", "0x0badcafe", 1);
    assert_ne!(
        attack_header, official_header,
        "approved header must contain the expected magic fixture point"
    );

    let attack_include = fixture.path("attack-include");
    write_file(&attack_include.join("libpq/oauth.h"), &attack_header);

    for variable in bindgen_extra_clang_args_names() {
        let run = run_generator(
            &fixture,
            &build_script,
            &official_pg_config,
            "bindgen-extra-clang-args",
            [(variable.as_str(), iquote_arg(&attack_include))],
            fixture.path("clean-cwd"),
        );
        assert_rejected(
            &run,
            "ambient bindgen or Clang configuration is not permitted",
            &format!("{variable} include injection"),
        );
    }

    for variable in AMBIENT_CLANG_ENV {
        let run = run_generator(
            &fixture,
            &build_script,
            &official_pg_config,
            "ambient-clang-env",
            [(variable, OsString::from("pggomtm-provenance-injection"))],
            fixture.path("clean-cwd"),
        );
        assert_rejected(
            &run,
            "ambient bindgen or Clang configuration is not permitted",
            &format!("{variable} ambient configuration"),
        );
    }

    let clang_probe_dir = fixture.path("path-clang-probe-bin");
    let clang_probe = clang_probe_dir.join("clang");
    let clang_probe_marker = fixture.path("path-clang-probe-executed");
    write_clang_probe(&clang_probe, &clang_probe_marker);
    let probe_path = format!(
        "{}:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        clang_probe_dir
            .to_str()
            .expect("clang probe path must be UTF-8")
    );
    let path_probe = run_generator(
        &fixture,
        &build_script,
        &official_pg_config,
        "path-clang-probe",
        [("PATH", OsString::from(probe_path))],
        fixture.path("clean-cwd"),
    );
    assert_generated_from_approved_header(&path_probe, "ambient PATH clang probe");
    assert!(
        !clang_probe_marker.exists(),
        "bindgen consulted an ambient PATH clang while deriving include paths"
    );

    let shadow_cwd = fixture.path("wrapper-shadow");
    write_file(&shadow_cwd.join("libpq/oauth.h"), &attack_header);
    let shadow = run_generator(
        &fixture,
        &build_script,
        &official_pg_config,
        "wrapper-shadow",
        std::iter::empty::<(&str, OsString)>(),
        shadow_cwd,
    );
    assert_generated_from_approved_header(&shadow, "repository-root header shadow");

    let quoted_include = fixture.path("quoted-include\"ignored");
    write_file(
        &quoted_include.join("postgres.h"),
        "/* approved-path injection fixture */\n",
    );
    write_file(&quoted_include.join("libpq/oauth.h"), &official_header);
    link_official_include_entries(&official_include, &quoted_include);
    let official_postgres = official_include.join("postgres.h");
    let official_postgres = official_postgres
        .to_str()
        .expect("approved postgres.h path must be UTF-8");
    assert!(
        !official_postgres.contains(['\n', '\r', '\"', '\\']),
        "approved postgres.h path must be safe for the fixture include"
    );
    write_file(
        &fixture.path("quoted-include"),
        &format!("#include \"{official_postgres}\"\n{attack_header}"),
    );
    let quoted_pg_config = fixture.path("quoted-include-bin/pg_config");
    write_pg_config(&quoted_pg_config, &quoted_include);
    let quoted = run_generator(
        &fixture,
        &build_script,
        &quoted_pg_config,
        "quoted-include",
        std::iter::empty::<(&str, OsString)>(),
        fixture.path("clean-cwd"),
    );
    assert_rejected(
        &quoted,
        "canonical server include path must be safe for Clang",
        "quoted canonical include path",
    );

    let pg_config_alias = fixture.path("pg-config-alias/pg_config");
    fs::create_dir_all(pg_config_alias.parent().expect("alias parent"))
        .expect("create pg_config alias parent");
    symlink(&official_pg_config, &pg_config_alias).expect("create pg_config alias");
    let alias = run_generator(
        &fixture,
        &build_script,
        &pg_config_alias,
        "pg-config-alias",
        std::iter::empty::<(&str, OsString)>(),
        fixture.path("clean-cwd"),
    );
    assert_rejected(
        &alias,
        "PGRX_PG_CONFIG_PATH must already be canonical",
        "non-canonical pg_config alias",
    );

    let include_alias = fixture.path("include-alias");
    symlink(&official_include, &include_alias).expect("create include alias");
    let include_alias_pg_config = fixture.path("include-alias-bin/pg_config");
    write_pg_config(&include_alias_pg_config, &include_alias);
    let alias = run_generator(
        &fixture,
        &build_script,
        &include_alias_pg_config,
        "include-alias",
        std::iter::empty::<(&str, OsString)>(),
        fixture.path("clean-cwd"),
    );
    assert_rejected(
        &alias,
        "server include directory must already be canonical",
        "non-canonical server include alias",
    );

    let non_utf8_dir = fixture.path(OsString::from_vec(b"non-utf8-\xff".to_vec()));
    let non_utf8_pg_config = non_utf8_dir.join("pg_config");
    write_pg_config(&non_utf8_pg_config, &official_include);
    let utf8_alias = fixture.path("non-utf8-pg-config-alias/pg_config");
    fs::create_dir_all(utf8_alias.parent().expect("UTF-8 alias parent"))
        .expect("create UTF-8 alias parent");
    symlink(&non_utf8_pg_config, &utf8_alias).expect("create non-UTF-8 target alias");
    let non_utf8 = run_generator(
        &fixture,
        &build_script,
        &utf8_alias,
        "non-utf8-pg-config",
        std::iter::empty::<(&str, OsString)>(),
        fixture.path("clean-cwd"),
    );
    assert_rejected(
        &non_utf8,
        "canonical pg_config path must be valid UTF-8",
        "non-UTF-8 canonical pg_config identity",
    );

    let digest_cases = [
        ("wrong-digest", format!("{official_header}\n")),
        (
            "missing-allowlist-symbol",
            official_header.replacen("ValidatorModuleState", "RemovedModuleState", 1),
        ),
        (
            "extra-allowlist-symbol",
            format!(
                "{official_header}\ntypedef struct UnexpectedOAuthItem {{ int value; }} UnexpectedOAuthItem;\n"
            ),
        ),
    ];
    for (name, header) in digest_cases {
        let include_dir = fixture.path(format!("{name}-include"));
        write_file(
            &include_dir.join("postgres.h"),
            "/* digest gate fixture */\n",
        );
        write_file(&include_dir.join("libpq/oauth.h"), &header);
        let pg_config = fixture.path(format!("{name}-bin/pg_config"));
        write_pg_config(&pg_config, &include_dir);
        let rejected = run_generator(
            &fixture,
            &build_script,
            &pg_config,
            name,
            std::iter::empty::<(&str, OsString)>(),
            fixture.path("clean-cwd"),
        );
        assert_rejected(&rejected, "OAuth header SHA-256 is not approved", name);
    }

    let wrong_target = run_generator(
        &fixture,
        &build_script,
        &official_pg_config,
        "wrong-target",
        [("TARGET", OsString::from("aarch64-unknown-linux-gnu"))],
        fixture.path("clean-cwd"),
    );
    assert_rejected(
        &wrong_target,
        "Cargo TARGET is not approved for this build variant",
        "unapproved Cargo target",
    );

    let success = run_generator(
        &fixture,
        &build_script,
        &official_pg_config,
        "approved-success",
        std::iter::empty::<(&str, OsString)>(),
        fixture.path("clean-cwd"),
    );
    assert_generated_from_approved_header(&success, "approved official header");
    let expected_bindings = fs::read(success.out_dir.join(BINDINGS_FILE))
        .expect("read approved final OAuth bindings bytes");
    assert_eq!(
        bindings_sha256(&expected_bindings),
        APPROVED_BINDINGS_SHA256,
        "approved generator final OUT_DIR bytes changed"
    );
    let build_identity = fs::read_to_string(success.out_dir.join(BUILD_IDENTITY_FILE))
        .expect("read canonical artifact identity");
    let build_identity_sha256 = format!("{:x}", Sha256::digest(build_identity.as_bytes()));

    let rustfmt_probe = fixture.path("rustfmt-env-probe/rustfmt");
    let rustfmt_marker = fixture.path("rustfmt-env-probe-executed");
    write_stateful_rustfmt_probe(&rustfmt_probe, &rustfmt_marker);
    let rustfmt_env = run_generator(
        &fixture,
        &build_script,
        &official_pg_config,
        "rustfmt-env-probe",
        [("RUSTFMT", rustfmt_probe.into_os_string())],
        fixture.path("clean-cwd"),
    );
    let rustfmt_env_violation =
        ambient_formatter_violation(&rustfmt_env, &expected_bindings, &rustfmt_marker);

    let path_rustfmt_dir = fixture.path("path-rustfmt-probe-bin");
    let path_rustfmt = path_rustfmt_dir.join("rustfmt");
    let path_rustfmt_marker = fixture.path("path-rustfmt-probe-executed");
    write_stateful_rustfmt_probe(&path_rustfmt, &path_rustfmt_marker);
    let path = format!(
        "{}:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        path_rustfmt_dir
            .to_str()
            .expect("rustfmt probe path must be UTF-8")
    );
    let path_rustfmt_run = run_generator(
        &fixture,
        &build_script,
        &official_pg_config,
        "path-rustfmt-probe",
        [("PATH", OsString::from(path))],
        fixture.path("clean-cwd"),
    );
    let path_rustfmt_violation =
        ambient_formatter_violation(&path_rustfmt_run, &expected_bindings, &path_rustfmt_marker);
    assert!(
        rustfmt_env_violation.is_none() && path_rustfmt_violation.is_none(),
        "ambient formatter changed the final OUT_DIR bytes after validation:\nRUSTFMT: {}\nPATH/rustfmt: {}",
        rustfmt_env_violation.as_deref().unwrap_or("isolated"),
        path_rustfmt_violation.as_deref().unwrap_or("isolated"),
    );

    let stdout = String::from_utf8_lossy(&success.output.stdout);
    assert!(
        stdout.contains(&format!(
            "cargo:rustc-env=PG_OAUTH_BINDINGS_SHA256={APPROVED_BINDINGS_SHA256}"
        )),
        "approved build omitted the final bindings digest: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "cargo:rustc-env=PGGOMTM_BUILD_IDENTITY_JSON={build_identity}"
        )) && stdout.contains(&format!(
            "cargo:rustc-env=PGGOMTM_BUILD_IDENTITY_SHA256={build_identity_sha256}"
        )),
        "approved build omitted the comparable artifact identity: {stdout}"
    );
    for variable in rerun_environment_names() {
        assert!(
            stdout.contains(&format!("cargo:rerun-if-env-changed={variable}")),
            "approved build omitted the Cargo rerun boundary for {variable}: {stdout}"
        );
    }
}

struct GeneratorRun {
    output: Output,
    out_dir: PathBuf,
}

fn run_generator<I, K>(
    fixture: &TempTree,
    build_script: &Path,
    pg_config: &Path,
    name: &str,
    extra_env: I,
    current_dir: PathBuf,
) -> GeneratorRun
where
    I: IntoIterator<Item = (K, OsString)>,
    K: AsRef<OsStr>,
{
    let out_dir = fixture.path(format!("out-{name}"));
    fs::create_dir_all(&out_dir).expect("create generator OUT_DIR");
    fs::create_dir_all(&current_dir).expect("create generator current directory");

    let mut command = Command::new(build_script);
    command
        .current_dir(current_dir)
        .env_clear()
        .env("OUT_DIR", &out_dir)
        .env("CARGO_HOME", "/usr/local/cargo")
        .env(
            "PATH",
            "/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        )
        .env("PGRX_PG_CONFIG_PATH", pg_config)
        .env("RUSTUP_HOME", "/usr/local/rustup")
        .env("CARGO_FEATURE_ABI_GATE", "1")
        .env("CARGO_FEATURE_PG18", "1")
        .env("TARGET", TARGET);
    for (key, value) in extra_env {
        command.env(key, value);
    }

    let output = command
        .output()
        .expect("execute the real Cargo build script");
    GeneratorRun { output, out_dir }
}

fn assert_rejected(run: &GeneratorRun, expected: &str, scenario: &str) {
    assert!(
        !run.output.status.success(),
        "{scenario} was accepted and generated {}",
        generated_summary(&run.out_dir)
    );
    let stderr = String::from_utf8_lossy(&run.output.stderr);
    assert!(
        stderr.contains(expected),
        "{scenario} failed for the wrong reason; expected {expected:?}, got: {stderr}"
    );
    assert!(
        !run.out_dir.join(BINDINGS_FILE).exists(),
        "{scenario} left generated bindings after rejection"
    );
}

fn assert_generated_from_approved_header(run: &GeneratorRun, scenario: &str) {
    assert!(
        run.output.status.success(),
        "{scenario} failed: {}",
        String::from_utf8_lossy(&run.output.stderr)
    );
    let generated =
        fs::read_to_string(run.out_dir.join(BINDINGS_FILE)).expect("read generated OAuth bindings");
    let compact = generated
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    assert!(
        compact.contains(&format!(
            "pubconstPG_OAUTH_VALIDATOR_MAGIC:u32={APPROVED_MAGIC};"
        )),
        "{scenario} did not generate the approved header magic: {generated}"
    );
    assert!(
        !compact.contains(&ATTACK_MAGIC.to_string()),
        "{scenario} accepted the shadow header instead of the hashed official header"
    );
}

fn ambient_formatter_violation(
    run: &GeneratorRun,
    expected_bindings: &[u8],
    marker: &Path,
) -> Option<String> {
    if !run.output.status.success() {
        return Some(format!(
            "generator failed before the final-byte check: {}",
            String::from_utf8_lossy(&run.output.stderr)
        ));
    }
    let actual_bindings = match fs::read(run.out_dir.join(BINDINGS_FILE)) {
        Ok(bindings) => bindings,
        Err(error) => return Some(format!("final OAuth bindings could not be read: {error}")),
    };
    if actual_bindings != expected_bindings || marker.exists() {
        return Some(format!(
            "formatter_executed={}, output={}",
            marker.exists(),
            generated_summary(&run.out_dir)
        ));
    }
    None
}

fn generated_summary(out_dir: &Path) -> String {
    fs::read_to_string(out_dir.join(BINDINGS_FILE))
        .map(|generated| generated.lines().take(4).collect::<Vec<_>>().join(" | "))
        .unwrap_or_else(|_| "no bindings".to_owned())
}

fn bindings_sha256(bindings: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bindings))
}

fn current_build_script() -> PathBuf {
    let test_executable = env::current_exe().expect("current test executable");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory");
    let build_root = profile_dir.join("build");
    let mut candidates = fs::read_dir(&build_root)
        .expect("read Cargo build directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("pggomtm-"))
        .map(|entry| entry.path().join("build-script-build"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "provenance gate requires a clean Cargo target; candidates: {candidates:?}"
    );
    candidates.pop().expect("one current build script")
}

fn canonical_env_path(name: &str) -> PathBuf {
    let value = env::var_os(name).unwrap_or_else(|| panic!("{name} must be set for native tests"));
    fs::canonicalize(value).unwrap_or_else(|_| panic!("canonicalize {name}"))
}

fn pg_config_include_dir(pg_config: &Path) -> PathBuf {
    let output = Command::new(pg_config)
        .arg("--includedir-server")
        .output()
        .expect("run approved pg_config");
    assert!(output.status.success(), "approved pg_config failed");
    let path = String::from_utf8(output.stdout)
        .expect("approved include path UTF-8")
        .trim_end()
        .to_owned();
    fs::canonicalize(path).expect("canonical approved include path")
}

fn write_pg_config(path: &Path, include_dir: &Path) {
    let include_dir = include_dir
        .to_str()
        .expect("fake pg_config include path must be UTF-8");
    assert!(
        !include_dir.contains(['\n', '\r', '\'']),
        "fixture path must be shell-safe"
    );
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  --version) printf '%s\\n' 'PostgreSQL 18.4' ;;\n  --includedir-server) printf '%s\\n' '{include_dir}' ;;\n  *) exit 64 ;;\nesac\n"
    );
    write_file(path, &script);
    let mut permissions = fs::metadata(path)
        .expect("fake pg_config metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fake pg_config executable");
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture file parent"))
        .expect("create fixture file parent");
    fs::write(path, contents).expect("write provenance fixture");
}

fn write_clang_probe(path: &Path, marker: &Path) {
    let marker = marker.to_str().expect("clang marker path must be UTF-8");
    assert!(
        !marker.contains(['\n', '\r', '\'']),
        "clang marker path must be shell-safe"
    );
    write_file(
        path,
        &format!("#!/bin/sh\n: > '{marker}'\nexec /usr/bin/clang \"$@\"\n"),
    );
    let mut permissions = fs::metadata(path)
        .expect("clang probe metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make clang probe executable");
}

fn write_stateful_rustfmt_probe(path: &Path, marker: &Path) {
    let marker = marker.to_str().expect("rustfmt marker path must be UTF-8");
    assert!(
        !marker.contains(['\n', '\r', '\'']),
        "rustfmt marker path must be shell-safe"
    );
    write_file(
        path,
        &format!(
            "#!/bin/sh\nset -eu\nif [ -e '{marker}' ]; then\n  /usr/local/cargo/bin/rustfmt \"$@\" | sed 's/{APPROVED_MAGIC}/{ATTACK_MAGIC}/g'\nelse\n  : > '{marker}'\n  exec /usr/local/cargo/bin/rustfmt \"$@\"\nfi\n"
        ),
    );
    let mut permissions = fs::metadata(path)
        .expect("rustfmt probe metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make rustfmt probe executable");
}

fn link_official_include_entries(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).expect("read approved include tree") {
        let entry = entry.expect("read approved include entry");
        let name = entry.file_name();
        if name == "postgres.h" {
            continue;
        }
        if name == "libpq" {
            for nested in fs::read_dir(entry.path()).expect("read approved libpq include tree") {
                let nested = nested.expect("read approved libpq include entry");
                if nested.file_name() == "oauth.h" {
                    continue;
                }
                symlink(
                    nested.path(),
                    destination.join("libpq").join(nested.file_name()),
                )
                .expect("link approved libpq include entry");
            }
            continue;
        }
        symlink(entry.path(), destination.join(name)).expect("link approved include entry");
    }
}

fn iquote_arg(include_dir: &Path) -> OsString {
    let include_dir = include_dir
        .to_str()
        .expect("attack include path must be UTF-8");
    OsString::from(format!("-iquote{include_dir}"))
}

fn bindgen_extra_clang_args_names() -> Vec<String> {
    vec![
        "BINDGEN_EXTRA_CLANG_ARGS".to_owned(),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{TARGET}"),
        format!("BINDGEN_EXTRA_CLANG_ARGS_{}", TARGET.replace('-', "_")),
    ]
}

fn rerun_environment_names() -> Vec<String> {
    let mut names = vec!["PGRX_PG_CONFIG_PATH".to_owned(), "TARGET".to_owned()];
    names.extend(bindgen_extra_clang_args_names());
    names.extend(AMBIENT_CLANG_ENV.into_iter().map(str::to_owned));
    names
}
