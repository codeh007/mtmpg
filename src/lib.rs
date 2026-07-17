#[cfg(any(feature = "abi-runtime-gate", feature = "pgx-oauth-gate"))]
use std::ffi::CStr;
#[cfg(feature = "pgx-oauth-gate")]
use std::ffi::CString;
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

#[cfg(not(any(
    feature = "abi-gate",
    feature = "abi-runtime-gate",
    feature = "pgx-oauth-gate"
)))]
use runtime_config::{ValidatorSnapshot, load_validator_snapshot};

pgrx::pg_module_magic!();

pub mod database_auth;
pub mod oauth_abi;
pub mod runtime_config;

pub use oauth_abi::{
    OAuthValidatorCallbacks, OAuthValidatorModuleInit, PG_OAUTH_BINDINGS_SHA256,
    PG_OAUTH_HEADER_SHA256, PG_OAUTH_VALIDATOR_MAGIC, ValidatorModuleResult, ValidatorModuleState,
    ValidatorShutdownCB, ValidatorStartupCB, ValidatorValidateCB,
};

pub const PG18_VERSION_NUM: i32 = 180_004;
pub const PGGOMTM_BUILD_IDENTITY_JSON: &str = env!("PGGOMTM_BUILD_IDENTITY_JSON");
pub const PGGOMTM_BUILD_IDENTITY_SHA256: &str = env!("PGGOMTM_BUILD_IDENTITY_SHA256");

#[used]
static EMBEDDED_BUILD_IDENTITY_JSON: &str = PGGOMTM_BUILD_IDENTITY_JSON;
#[used]
static EMBEDDED_BUILD_IDENTITY_SHA256: &str = PGGOMTM_BUILD_IDENTITY_SHA256;

#[cfg(feature = "abi-runtime-gate")]
const ABI_RUNTIME_PANIC_SENTINEL: usize = 1;
#[cfg(feature = "abi-runtime-gate")]
const ABI_RUNTIME_ERROR_SENTINEL: usize = 2;

#[cfg(feature = "pgx-oauth-gate")]
const PGX_OAUTH_GATE_JWKS: &str = r#"{"keys":[{"kty":"EC","crv":"P-256","alg":"ES256","use":"sig","key_ops":["verify"],"kid":"candidate-es256-pgx-gate","x":"HhhTL9R1TALzBB2cdc6zO4P_2BrHzk_ogsyxyYvFiW4","y":"pGwxHE4v9A3ZajZT5uRURdMt_khuztdcepDGoYiBwKM"}]}"#;

#[cfg(feature = "pgx-oauth-gate")]
pub fn verify_pgx_gate_token(
    token: &str,
    requested_role: &str,
    now: i64,
) -> Result<String, database_auth::JwtValidationError> {
    let policy = database_auth::DatabaseTokenPolicy::new(
        "https://candidate.example.test/oauth/database",
        "https://candidate.example.test/resources/database/gomtm-test",
    )?;
    let verifier = database_auth::DatabaseTokenVerifier::from_jwks(PGX_OAUTH_GATE_JWKS, policy)?;
    Ok(verifier.verify(token, requested_role, now)?.authn_id)
}

static OAUTH_CALLBACKS: OAuthValidatorCallbacks = OAuthValidatorCallbacks {
    magic: PG_OAUTH_VALIDATOR_MAGIC,
    startup_cb: Some(validator_startup),
    shutdown_cb: Some(validator_shutdown),
    validate_cb: Some(validate_token),
};

#[must_use]
pub const fn server_version_is_supported(server_version: i32) -> bool {
    server_version / 10_000 == 18
}

#[must_use]
pub fn oauth_callbacks() -> &'static OAuthValidatorCallbacks {
    &OAUTH_CALLBACKS
}

#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validator_startup(state: *mut ValidatorModuleState) {
    if state.is_null() {
        #[cfg(feature = "abi-gate")]
        panic!("pggomtm received a null validator state");

        #[cfg(not(feature = "abi-gate"))]
        pgrx::error!("pggomtm received a null validator state");
    }

    let state = unsafe { &mut *state };
    #[cfg(feature = "abi-runtime-gate")]
    let initial_private_data = state.private_data;

    #[cfg(not(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    )))]
    if !state.private_data.is_null() {
        pgrx::error!("pggomtm validator state was already initialized");
    }

    state.private_data = ptr::null_mut();

    #[cfg(feature = "abi-runtime-gate")]
    match initial_private_data.addr() {
        ABI_RUNTIME_PANIC_SENTINEL => panic!("pggomtm ABI runtime startup panic gate"),
        ABI_RUNTIME_ERROR_SENTINEL => pgrx::error!("pggomtm ABI runtime startup error gate"),
        _ => {}
    }

    if !server_version_is_supported(state.sversion) {
        #[cfg(feature = "abi-gate")]
        panic!("pggomtm requires PostgreSQL 18");

        #[cfg(not(feature = "abi-gate"))]
        pgrx::error!("pggomtm requires PostgreSQL 18");
    }

    #[cfg(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    ))]
    {
        state.private_data = state as *mut ValidatorModuleState as *mut c_void;
    }

    #[cfg(not(any(
        feature = "abi-gate",
        feature = "abi-runtime-gate",
        feature = "pgx-oauth-gate"
    )))]
    {
        let snapshot = load_validator_snapshot()
            .unwrap_or_else(|error| pgrx::error!("pggomtm validator startup failed: {error}"));
        state.private_data = Box::into_raw(Box::new(snapshot)).cast::<c_void>();
    }
}

#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validator_shutdown(state: *mut ValidatorModuleState) {
    if let Some(state) = unsafe { state.as_mut() } {
        #[cfg(any(
            feature = "abi-runtime-gate",
            not(any(
                feature = "abi-gate",
                feature = "abi-runtime-gate",
                feature = "pgx-oauth-gate"
            ))
        ))]
        let initial_private_data = state.private_data;
        state.private_data = ptr::null_mut();

        #[cfg(feature = "abi-runtime-gate")]
        match initial_private_data.addr() {
            ABI_RUNTIME_PANIC_SENTINEL => panic!("pggomtm ABI runtime shutdown panic gate"),
            ABI_RUNTIME_ERROR_SENTINEL => {
                pgrx::error!("pggomtm ABI runtime shutdown error gate")
            }
            _ => {}
        }

        #[cfg(not(any(
            feature = "abi-gate",
            feature = "abi-runtime-gate",
            feature = "pgx-oauth-gate"
        )))]
        if !initial_private_data.is_null() {
            drop(unsafe { Box::from_raw(initial_private_data.cast::<ValidatorSnapshot>()) });
        }
    }
}

#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validate_token(
    state: *const ValidatorModuleState,
    token: *const c_char,
    role: *const c_char,
    result: *mut ValidatorModuleResult,
) -> bool {
    if result.is_null() {
        return false;
    }

    unsafe {
        (*result).authorized = false;
        (*result).authn_id = ptr::null_mut();
    }

    let validation = catch_unwind(AssertUnwindSafe(|| {
        let Some(state) = (unsafe { state.as_ref() }) else {
            return false;
        };

        if token.is_null() || role.is_null() || !server_version_is_supported(state.sversion) {
            return false;
        }

        #[cfg(any(
            feature = "abi-gate",
            feature = "abi-runtime-gate",
            feature = "pgx-oauth-gate"
        ))]
        if state.private_data != state as *const ValidatorModuleState as *mut c_void {
            return false;
        }

        #[cfg(not(any(
            feature = "abi-gate",
            feature = "abi-runtime-gate",
            feature = "pgx-oauth-gate"
        )))]
        if state.private_data.is_null() {
            return false;
        }

        #[cfg(feature = "abi-runtime-gate")]
        {
            let token = unsafe { CStr::from_ptr(token) };

            if token == c"pggomtm-abi-panic" {
                panic!("pggomtm ABI runtime panic gate");
            }

            if token == c"pggomtm-abi-error" {
                pgrx::error!("pggomtm ABI runtime validate error gate");
            }

            if token == c"pggomtm-abi-allocator" {
                let result = unsafe { &mut *result };
                result.authn_id = unsafe { pgrx::pg_sys::pstrdup(token.as_ptr()) };
                return !result.authn_id.is_null();
            }
        }

        #[cfg(feature = "pgx-oauth-gate")]
        {
            let token = unsafe { CStr::from_ptr(token) };
            let role = unsafe { CStr::from_ptr(role) };
            let (Ok(token), Ok(role)) = (token.to_str(), role.to_str()) else {
                return true;
            };
            let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
                return false;
            };
            let now = i64::try_from(now.as_secs()).unwrap_or(i64::MAX);
            let Ok(authn_id) = verify_pgx_gate_token(token, role, now) else {
                return true;
            };
            let Ok(authn_id) = CString::new(authn_id) else {
                return false;
            };
            let result = unsafe { &mut *result };
            result.authn_id = unsafe { pgrx::pg_sys::pstrdup(authn_id.as_ptr()) };
            result.authorized = !result.authn_id.is_null();
            result.authorized
        }

        #[cfg(not(feature = "pgx-oauth-gate"))]
        true
    }));

    match validation {
        Ok(authorized) => authorized,
        Err(error)
            if error.is::<pgrx::pg_sys::panic::CaughtError>()
                || error.is::<pgrx::pg_sys::panic::ErrorReport>()
                || error.is::<pgrx::pg_sys::panic::ErrorReportWithLevel>() =>
        {
            std::panic::resume_unwind(error)
        }
        Err(_) => false,
    }
}

#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
#[unsafe(no_mangle)]
pub extern "C-unwind" fn _PG_oauth_validator_module_init() -> *const OAuthValidatorCallbacks {
    oauth_callbacks() as *const OAuthValidatorCallbacks
}

const _: OAuthValidatorModuleInit = Some(_PG_oauth_validator_module_init);

#[cfg(feature = "abi-gate")]
#[must_use]
pub fn panic_boundary_for_gate() -> bool {
    catch_unwind(AssertUnwindSafe(|| panic!("pggomtm ABI panic gate"))).is_ok()
}
