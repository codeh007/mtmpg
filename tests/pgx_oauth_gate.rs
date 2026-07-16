#![cfg(feature = "pgx-oauth-gate")]

use jaws::Token;
use p256::ecdsa::{Signature, SigningKey};
use pggomtm::database_auth::{
    AuthMethod, DatabaseProfile, DatabaseTokenClaims, JwtValidationError,
};
use pggomtm::verify_pgx_gate_token;

const NOW: i64 = 1_800_000_000;

fn signed_gate_token() -> String {
    let claims = DatabaseTokenClaims {
        issuer: "https://candidate.example.test/oauth/database".into(),
        audience: "https://candidate.example.test/resources/database/gomtm-test".into(),
        subject: "usr_pgx_gate".into(),
        issued_at: NOW,
        expires_at: NOW + 120,
        token_id: "jti_pgx_gate".into(),
        scope: "database".into(),
        delegation_id: "dlg_pgx_gate".into(),
        auth_method: AuthMethod::OAuth,
        authority_version: 1,
        db_profile: DatabaseProfile::Ordinary,
        db_role: DatabaseProfile::Ordinary.database_role().into(),
        client_id: Some("cli_pgx_gate".into()),
        credential_id: None,
    };
    let key = SigningKey::from_slice(&[7_u8; 32]).expect("fixed gate signing key");
    let mut token = Token::compact((), claims);
    *token.header_mut().key_id() = Some("candidate-es256-pgx-gate".into());
    *token.header_mut().r#type() = Some("JWT".into());
    token
        .sign::<_, Signature>(&key)
        .expect("sign gate token")
        .rendered()
        .expect("render gate token")
}

#[test]
fn gate_verifier_returns_versioned_identity_for_matching_role() {
    let token = signed_gate_token();
    assert_eq!(
        verify_pgx_gate_token(&token, DatabaseProfile::Ordinary.database_role(), NOW + 1),
        Ok("pggomtm:v1;u=usr_pgx_gate;actor=client:cli_pgx_gate;d=dlg_pgx_gate;m=oauth;a=1;p=ordinary".into())
    );
}

#[test]
fn gate_verifier_rejects_role_mismatch_and_tampering() {
    let token = signed_gate_token();
    assert_eq!(
        verify_pgx_gate_token(
            &token,
            DatabaseProfile::BusinessAdmin.database_role(),
            NOW + 1,
        ),
        Err(JwtValidationError::RequestedRoleMismatch)
    );

    let mut tampered = token.into_bytes();
    let last = tampered.last_mut().expect("signature byte");
    *last = if *last == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(tampered).expect("compact token");
    assert!(matches!(
        verify_pgx_gate_token(
            &tampered,
            DatabaseProfile::Ordinary.database_role(),
            NOW + 1,
        ),
        Err(JwtValidationError::InvalidSignature | JwtValidationError::InvalidToken)
    ));
}
