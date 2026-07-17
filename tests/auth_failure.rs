use std::collections::BTreeSet;

use pggomtm::auth_failure::AuthenticationFailureReason;
use pggomtm::database_auth::JwtValidationError;
use pggomtm::runtime_config::RuntimeConfigError;

#[test]
fn versioned_reason_codes_are_unique_and_stable() {
    let expected = [
        "pggomtm-auth/v1/config-missing",
        "pggomtm-auth/v1/jwks-missing",
        "pggomtm-auth/v1/config-too-large",
        "pggomtm-auth/v1/jwks-too-large",
        "pggomtm-auth/v1/material-file-type-unsafe",
        "pggomtm-auth/v1/material-permissions-unsafe",
        "pggomtm-auth/v1/material-publication-layout-unsafe",
        "pggomtm-auth/v1/config-invalid",
        "pggomtm-auth/v1/resources-invalid",
        "pggomtm-auth/v1/jwks-invalid",
        "pggomtm-auth/v1/jwks-duplicate-kid",
        "pggomtm-auth/v1/token-policy-invalid",
        "pggomtm-auth/v1/token-invalid",
        "pggomtm-auth/v1/token-header-invalid",
        "pggomtm-auth/v1/token-kid-unknown",
        "pggomtm-auth/v1/token-signature-invalid",
        "pggomtm-auth/v1/token-claims-invalid",
        "pggomtm-auth/v1/token-role-mismatch",
        "pggomtm-auth/v1/identity-invalid",
        "pggomtm-auth/v1/callback-input-invalid",
        "pggomtm-auth/v1/callback-state-invalid",
        "pggomtm-auth/v1/postgres-major-unsupported",
        "pggomtm-auth/v1/internal-panic",
        "pggomtm-auth/v1/postgres-error",
    ];
    let actual = AuthenticationFailureReason::ALL.map(AuthenticationFailureReason::code);

    assert_eq!(actual, expected);
    assert_eq!(
        actual.into_iter().collect::<BTreeSet<_>>().len(),
        actual.len()
    );
}

#[test]
fn jwt_errors_map_to_the_versioned_closed_reason_set() {
    let mappings = [
        (
            JwtValidationError::InvalidPolicy,
            "pggomtm-auth/v1/token-policy-invalid",
        ),
        (
            JwtValidationError::InvalidJwks,
            "pggomtm-auth/v1/jwks-invalid",
        ),
        (
            JwtValidationError::DuplicateKeyId,
            "pggomtm-auth/v1/jwks-duplicate-kid",
        ),
        (
            JwtValidationError::InvalidToken,
            "pggomtm-auth/v1/token-invalid",
        ),
        (
            JwtValidationError::InvalidHeader,
            "pggomtm-auth/v1/token-header-invalid",
        ),
        (
            JwtValidationError::UnknownKeyId,
            "pggomtm-auth/v1/token-kid-unknown",
        ),
        (
            JwtValidationError::InvalidSignature,
            "pggomtm-auth/v1/token-signature-invalid",
        ),
        (
            JwtValidationError::InvalidClaims,
            "pggomtm-auth/v1/token-claims-invalid",
        ),
        (
            JwtValidationError::RequestedRoleMismatch,
            "pggomtm-auth/v1/token-role-mismatch",
        ),
        (
            JwtValidationError::InvalidIdentity,
            "pggomtm-auth/v1/identity-invalid",
        ),
    ];

    for (error, expected) in mappings {
        assert_eq!(error.reason_code(), expected);
    }
}

#[test]
fn runtime_errors_map_to_the_versioned_closed_reason_set() {
    let mappings = [
        (
            RuntimeConfigError::ConfigMissing,
            "pggomtm-auth/v1/config-missing",
        ),
        (
            RuntimeConfigError::JwksMissing,
            "pggomtm-auth/v1/jwks-missing",
        ),
        (
            RuntimeConfigError::ConfigTooLarge,
            "pggomtm-auth/v1/config-too-large",
        ),
        (
            RuntimeConfigError::JwksTooLarge,
            "pggomtm-auth/v1/jwks-too-large",
        ),
        (
            RuntimeConfigError::UnsafeFileType,
            "pggomtm-auth/v1/material-file-type-unsafe",
        ),
        (
            RuntimeConfigError::UnsafePermissions,
            "pggomtm-auth/v1/material-permissions-unsafe",
        ),
        (
            RuntimeConfigError::UnsafePublicationLayout,
            "pggomtm-auth/v1/material-publication-layout-unsafe",
        ),
        (
            RuntimeConfigError::InvalidConfig,
            "pggomtm-auth/v1/config-invalid",
        ),
        (
            RuntimeConfigError::InvalidResources,
            "pggomtm-auth/v1/resources-invalid",
        ),
        (
            RuntimeConfigError::InvalidJwks,
            "pggomtm-auth/v1/jwks-invalid",
        ),
        (
            RuntimeConfigError::DuplicateKeyId,
            "pggomtm-auth/v1/jwks-duplicate-kid",
        ),
    ];

    for (error, expected) in mappings {
        assert_eq!(error.reason_code(), expected);
    }
}
