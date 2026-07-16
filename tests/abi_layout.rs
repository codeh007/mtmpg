use std::ffi::CString;
use std::mem::{offset_of, size_of};
use std::ptr;

use pggomtm::{
    OAuthValidatorCallbacks, PG_OAUTH_VALIDATOR_MAGIC, PG18_VERSION_NUM, ValidatorModuleResult,
    ValidatorModuleState, oauth_callbacks, server_version_is_supported,
};

#[test]
fn rust_layout_matches_postgresql_18_4_oauth_header() {
    assert_eq!(PG18_VERSION_NUM, 180_004);
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
    let role = CString::new("gomtm_candidate_ordinary").expect("role");
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
fn exact_minor_gate_rejects_other_pg18_servers() {
    assert!(server_version_is_supported(PG18_VERSION_NUM));
    assert!(!server_version_is_supported(180_003));
    assert!(!server_version_is_supported(180_005));
    assert!(!server_version_is_supported(170_007));
}

#[cfg(feature = "abi-gate")]
#[test]
fn panic_boundary_converts_unwind_to_internal_failure() {
    assert!(!pggomtm::panic_boundary_for_gate());
}
