use std::ffi::{CString, c_char};
use std::mem::{offset_of, size_of};
#[cfg(feature = "abi-gate")]
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use pggomtm::{
    OAuthValidatorCallbacks, OAuthValidatorModuleInit, PG_OAUTH_VALIDATOR_MAGIC, PG18_VERSION_NUM,
    ValidatorModuleResult, ValidatorModuleState, ValidatorShutdownCB, ValidatorStartupCB,
    ValidatorValidateCB, oauth_callbacks, server_version_is_supported,
};

#[test]
fn rust_layout_matches_the_current_postgresql_18_oauth_header() {
    assert_eq!(PG18_VERSION_NUM / 10_000, 18);
    assert_eq!(PG_OAUTH_VALIDATOR_MAGIC, 0x2025_0220);

    assert_eq!(size_of::<ValidatorModuleState>(), 16);
    assert_eq!(offset_of!(ValidatorModuleState, sversion), 0);
    assert_eq!(offset_of!(ValidatorModuleState, private_data), 8);

    assert_eq!(size_of::<ValidatorModuleResult>(), 16);
    assert_eq!(offset_of!(ValidatorModuleResult, authorized), 0);
    assert_eq!(offset_of!(ValidatorModuleResult, authn_id), 8);

    assert_eq!(size_of::<OAuthValidatorCallbacks>(), 32);
    assert_eq!(offset_of!(OAuthValidatorCallbacks, magic), 0);
    assert_eq!(offset_of!(OAuthValidatorCallbacks, startup_cb), 8);
    assert_eq!(offset_of!(OAuthValidatorCallbacks, shutdown_cb), 16);
    assert_eq!(offset_of!(OAuthValidatorCallbacks, validate_cb), 24);
}

#[test]
fn exported_init_uses_the_generated_postgresql_signature() {
    let init: OAuthValidatorModuleInit = Some(pggomtm::_PG_oauth_validator_module_init);
    assert!(init.is_some());
}

#[test]
fn generated_callback_types_match_the_official_surface() {
    unsafe extern "C-unwind" fn startup(_state: *mut ValidatorModuleState) {}
    unsafe extern "C-unwind" fn shutdown(_state: *mut ValidatorModuleState) {}
    unsafe extern "C-unwind" fn validate(
        _state: *const ValidatorModuleState,
        _token: *const c_char,
        _role: *const c_char,
        _result: *mut ValidatorModuleResult,
    ) -> bool {
        false
    }

    let startup: ValidatorStartupCB = Some(startup);
    let shutdown: ValidatorShutdownCB = Some(shutdown);
    let validate: ValidatorValidateCB = Some(validate);
    assert!(startup.is_some());
    assert!(shutdown.is_some());
    assert!(validate.is_some());
}

#[test]
fn callback_table_initializes_and_fails_closed_before_jwt_gate() {
    let callbacks = oauth_callbacks();
    assert_eq!(callbacks.magic, PG_OAUTH_VALIDATOR_MAGIC);

    let startup = callbacks.startup_cb.expect("startup callback");
    let shutdown = callbacks.shutdown_cb.expect("shutdown callback");
    let validate = callbacks.validate_cb.expect("validate callback");
    let mut state = ValidatorModuleState {
        sversion: PG18_VERSION_NUM,
        private_data: ptr::null_mut(),
    };
    let token = CString::new("header.payload.signature").expect("token");
    let role = CString::new("ordinary").expect("role");
    let mut result = ValidatorModuleResult {
        authorized: true,
        authn_id: ptr::dangling_mut(),
    };

    unsafe { startup(&mut state) };
    assert!(!state.private_data.is_null());
    assert!(unsafe { validate(&state, token.as_ptr(), role.as_ptr(), &mut result) });
    assert!(!result.authorized);
    assert!(result.authn_id.is_null());
    unsafe { shutdown(&mut state) };
    assert!(state.private_data.is_null());
}

#[test]
fn pg18_stable_line_versions_are_supported() {
    for server_version in [180_000, 180_001, 180_999, 189_999] {
        assert!(
            server_version_is_supported(server_version),
            "PG18 version {server_version} must pass the runtime major gate"
        );
    }
}

#[test]
fn other_postgresql_majors_are_rejected() {
    for server_version in [170_999, 190_000] {
        assert!(
            !server_version_is_supported(server_version),
            "non-PG18 version {server_version} must fail the runtime major gate"
        );
    }
}

#[cfg(feature = "abi-gate")]
#[test]
fn panic_boundary_converts_unwind_to_internal_failure() {
    assert!(!pggomtm::panic_boundary_for_gate());
}

#[cfg(feature = "abi-gate")]
#[test]
fn callback_null_inputs_fail_closed_without_leaking_result_state() {
    let callbacks = oauth_callbacks();
    let startup = callbacks.startup_cb.expect("startup callback");
    let shutdown = callbacks.shutdown_cb.expect("shutdown callback");
    let validate = callbacks.validate_cb.expect("validate callback");

    assert!(
        catch_unwind(AssertUnwindSafe(|| unsafe { startup(ptr::null_mut()) })).is_err(),
        "abi-gate startup must reject a null state"
    );
    unsafe { shutdown(ptr::null_mut()) };

    let mut state = ValidatorModuleState {
        sversion: PG18_VERSION_NUM,
        private_data: ptr::null_mut(),
    };
    let token = CString::new("header.payload.signature").expect("token");
    let role = CString::new("ordinary").expect("role");
    unsafe { startup(&mut state) };

    for (state_ptr, token_ptr, role_ptr) in [
        (ptr::null(), token.as_ptr(), role.as_ptr()),
        (&state, ptr::null(), role.as_ptr()),
        (&state, token.as_ptr(), ptr::null()),
    ] {
        let mut result = ValidatorModuleResult {
            authorized: true,
            authn_id: ptr::dangling_mut(),
        };
        assert!(!unsafe { validate(state_ptr, token_ptr, role_ptr, &mut result) });
        assert!(!result.authorized);
        assert!(result.authn_id.is_null());
    }

    assert!(!unsafe { validate(&state, token.as_ptr(), role.as_ptr(), ptr::null_mut(),) });
    unsafe { shutdown(&mut state) };
    assert!(state.private_data.is_null());
}
