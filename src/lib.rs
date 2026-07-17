#[cfg(any(feature = "abi-runtime-gate", feature = "pgx-oauth-gate"))]
use std::ffi::CStr;
#[cfg(feature = "pgx-oauth-gate")]
use std::ffi::CString;
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

pgrx::pg_module_magic!();

pub mod database_auth;
pub mod oauth_abi;

pub use oauth_abi::{
    OAuthValidatorCallbacks, OAuthValidatorModuleInit, PG_OAUTH_HEADER_SHA256,
    PG_OAUTH_VALIDATOR_MAGIC, ValidatorModuleResult, ValidatorModuleState, ValidatorShutdownCB,
    ValidatorStartupCB, ValidatorValidateCB,
};

pub const PG18_VERSION_NUM: i32 = 180_004;

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
    server_version == PG18_VERSION_NUM
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
    state.private_data = ptr::null_mut();

    if !server_version_is_supported(state.sversion) {
        #[cfg(feature = "abi-gate")]
        panic!("pggomtm requires PostgreSQL 18.4");

        #[cfg(not(feature = "abi-gate"))]
        pgrx::error!("pggomtm requires PostgreSQL 18.4");
    }

    state.private_data = state as *mut ValidatorModuleState as *mut c_void;
}

#[cfg_attr(not(feature = "abi-gate"), pgrx::pg_guard)]
unsafe extern "C-unwind" fn validator_shutdown(state: *mut ValidatorModuleState) {
    if let Some(state) = unsafe { state.as_mut() } {
        state.private_data = ptr::null_mut();
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

    catch_unwind(AssertUnwindSafe(|| {
        let Some(state) = (unsafe { state.as_ref() }) else {
            return false;
        };

        if token.is_null() || role.is_null() || !server_version_is_supported(state.sversion) {
            return false;
        }

        if state.private_data != state as *const ValidatorModuleState as *mut c_void {
            return false;
        }

        #[cfg(feature = "abi-runtime-gate")]
        {
            let token = unsafe { CStr::from_ptr(token) };

            if token == c"pggomtm-abi-panic" {
                panic!("pggomtm ABI runtime panic gate");
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
    }))
    .unwrap_or(false)
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
