use std::collections::BTreeMap;

use base64ct::{Base64UrlUnpadded, Encoding};
use jaws::Token;
use jaws::key::JsonWebKey;
use p256::ecdsa::{Signature, SigningKey};
use pggomtm::database_auth::{
    AuthenticatedIdentity, DatabaseProfile, DatabaseTokenClaims, DatabaseTokenPolicy,
    DatabaseTokenVerifier, JwtValidationError, MAX_AUTHN_ID_BYTES, MAX_TOKEN_TTL_SECONDS,
    MIN_TOKEN_TTL_SECONDS, decode_authn_id, decode_system_user,
};
use serde::Serialize;
use serde_json::{Value, json};

const ISSUER: &str = "https://candidate.example.test/oauth/database";
const AUDIENCE: &str = "https://candidate.example.test/resources/database/gomtm-test";
const ISSUER_HOST: &str = "candidate.example.test";
const NOW: i64 = 1_800_000_000;
const KID: &str = "candidate-es256-2026-07";

fn signing_key() -> SigningKey {
    SigningKey::from_slice(&[7_u8; 32]).expect("fixed test signing key")
}

fn jwk_value(key: &SigningKey, kid: &str) -> Value {
    let mut value =
        serde_json::to_value(JsonWebKey::build(key.verifying_key())).expect("serialize public JWK");
    let object = value.as_object_mut().expect("JWK object");
    object.insert("alg".into(), json!("ES256"));
    object.insert("key_ops".into(), json!(["verify"]));
    object.insert("kid".into(), json!(kid));
    object.insert("use".into(), json!("sig"));
    value
}

fn jwks_with(keys: Vec<Value>) -> String {
    serde_json::to_string(&json!({ "keys": keys })).expect("serialize JWKS")
}

fn verifier(jwks: &str) -> DatabaseTokenVerifier {
    let policy = DatabaseTokenPolicy::new(ISSUER, AUDIENCE).expect("absolute token policy");
    DatabaseTokenVerifier::from_jwks(jwks, policy).expect("valid verifier")
}

#[test]
fn policy_requires_distinct_absolute_https_resources() {
    assert_eq!(
        DatabaseTokenPolicy::new("/relative", AUDIENCE),
        Err(JwtValidationError::InvalidPolicy)
    );
    assert_eq!(
        DatabaseTokenPolicy::new("http://candidate.example.test/issuer", AUDIENCE),
        Err(JwtValidationError::InvalidPolicy)
    );
    assert_eq!(
        DatabaseTokenPolicy::new(ISSUER, ISSUER),
        Err(JwtValidationError::InvalidPolicy)
    );
}

fn valid_claims() -> DatabaseTokenClaims {
    DatabaseTokenClaims {
        issuer: ISSUER.into(),
        audience: AUDIENCE.into(),
        subject: "usr_01J00000000000000000000000".into(),
        issued_at: NOW,
        expires_at: NOW + 120,
        token_id: "jti_01J00000000000000000000000".into(),
        scope: "database".into(),
        profile: DatabaseProfile::Ordinary,
    }
}

fn ordinary_identity() -> AuthenticatedIdentity {
    AuthenticatedIdentity {
        user_id: "usr_01J00000000000000000000000".into(),
        profile: DatabaseProfile::Ordinary,
        issuer_host: ISSUER_HOST.into(),
    }
}

fn sign_payload(payload: impl Serialize, kid: &str, key: &SigningKey) -> String {
    let mut token = Token::compact((), payload);
    *token.header_mut().key_id() = Some(kid.into());
    token
        .sign::<_, Signature>(key)
        .expect("sign token")
        .rendered()
        .expect("render compact token")
}

fn mutate_header(token: &str, mutate: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
    let mut segments = token.split('.').map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(segments.len(), 3);
    let decoded = Base64UrlUnpadded::decode_vec(&segments[0]).expect("decode header");
    let mut header: Value = serde_json::from_slice(&decoded).expect("header JSON");
    mutate(header.as_object_mut().expect("header object"));
    segments[0] = Base64UrlUnpadded::encode_string(
        &serde_json::to_vec(&header).expect("serialize mutated header"),
    );
    segments.join(".")
}

#[test]
fn valid_es256_token_round_trips_versioned_identity() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let claims = valid_claims();
    let token = sign_payload(claims.clone(), KID, &key);

    let verified = verifier
        .verify(&token, claims.profile.database_role(), NOW + 1)
        .expect("valid database token");

    assert_eq!(verified.claims, claims);
    assert_eq!(verified.identity, ordinary_identity());
    assert_eq!(
        verified.authn_id,
        "candidate.example.test:v1;u=usr_01J00000000000000000000000;p=ordinary"
    );
    assert_eq!(
        decode_authn_id(&verified.authn_id),
        Ok(verified.identity.clone())
    );
    assert_eq!(
        decode_system_user(&format!("oauth:{}", verified.authn_id)),
        Ok(verified.identity)
    );
    assert!(verified.authn_id.len() <= MAX_AUTHN_ID_BYTES);
}

#[test]
fn every_profile_and_ttl_boundary_is_valid() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));

    for (profile_index, profile) in [
        DatabaseProfile::Ordinary,
        DatabaseProfile::BusinessAdmin,
        DatabaseProfile::DatabaseDeveloper,
    ]
    .into_iter()
    .enumerate()
    {
        let mut claims = valid_claims();
        claims.profile = profile;
        claims.expires_at = NOW
            + if profile_index % 2 == 0 {
                MIN_TOKEN_TTL_SECONDS
            } else {
                MAX_TOKEN_TTL_SECONDS
            };
        let token = sign_payload(claims.clone(), KID, &key);

        let verified = verifier
            .verify(&token, profile.database_role(), NOW)
            .expect("closed profile must verify");
        assert_eq!(verified.claims, claims);
        assert_eq!(verified.identity.profile, profile);
        assert_eq!(
            decode_authn_id(&verified.authn_id),
            Ok(verified.identity.clone())
        );
        assert_eq!(
            decode_system_user(&format!("oauth:{}", verified.authn_id)),
            Ok(verified.identity)
        );
    }
}

#[test]
fn removed_legacy_claims_and_unknown_profile_fail_closed() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_claims();
    let expected_role = base.profile.database_role();

    for (field, value) in [
        ("delegation_id", json!("dlg_01J00000000000000000000000")),
        ("auth_method", json!("oauth")),
        ("authority_version", json!(7)),
        ("client_id", json!("cli_01J00000000000000000000000")),
        ("credential_id", json!("crd_01J00000000000000000000000")),
        ("db_role", json!("ordinary")),
        ("db_profile", json!("ordinary")),
    ] {
        let mut claims = serde_json::to_value(base.clone()).expect("claims JSON");
        claims
            .as_object_mut()
            .expect("claims object")
            .insert(field.into(), value);
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, expected_role, NOW),
            Err(JwtValidationError::InvalidToken),
            "legacy field {field} must be denied as unknown"
        );
    }

    for profile in [
        "cluster-admin",
        "business-admin",
        "database-developer",
        "gomtm_ordinary",
    ] {
        let mut claims = serde_json::to_value(base.clone()).expect("claims JSON");
        claims
            .as_object_mut()
            .expect("claims object")
            .insert("profile".into(), json!(profile));
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, expected_role, NOW),
            Err(JwtValidationError::InvalidToken),
            "unknown profile {profile} must fail closed"
        );
    }
}

#[test]
fn subject_and_jti_id_matrix_fails_closed() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_claims();
    let expected_role = base.profile.database_role();

    let maximum_id = "x".repeat(64);
    let mut boundary = base.clone();
    boundary.subject = maximum_id.clone();
    boundary.token_id = maximum_id.clone();
    let token = sign_payload(boundary, KID, &key);
    verifier
        .verify(&token, expected_role, NOW)
        .expect("64-byte IDs are inside the contract boundary");

    let invalid_ids = [
        String::new(),
        "contains:delimiter".into(),
        "含有非ASCII".into(),
        "x".repeat(65),
    ];
    for invalid_id in invalid_ids {
        for (field, expected) in [
            ("sub", JwtValidationError::InvalidIdentity),
            ("jti", JwtValidationError::InvalidClaims),
        ] {
            let mut claims = serde_json::to_value(base.clone()).expect("claims JSON");
            claims
                .as_object_mut()
                .expect("claims object")
                .insert(field.into(), json!(invalid_id.clone()));
            let token = sign_payload(claims, KID, &key);
            assert_eq!(
                verifier.verify(&token, expected_role, NOW),
                Err(expected),
                "field {field} must reject ID {invalid_id:?}"
            );
        }
    }
}

#[test]
fn jwks_rejects_duplicate_private_or_non_es256_keys() {
    let key = signing_key();
    let valid = jwk_value(&key, KID);
    let policy = || DatabaseTokenPolicy::new(ISSUER, AUDIENCE).expect("policy");

    assert!(matches!(
        DatabaseTokenVerifier::from_jwks(&jwks_with(vec![valid.clone(), valid.clone()]), policy(),),
        Err(JwtValidationError::DuplicateKeyId)
    ));

    for (field, value) in [
        ("d", json!("private-material-must-not-load")),
        ("kty", json!("RSA")),
        ("crv", json!("P-384")),
        ("alg", json!("ES384")),
        ("use", json!("enc")),
        ("key_ops", json!(["sign"])),
        ("x", json!("not-base64url=")),
    ] {
        let mut invalid = valid.clone();
        invalid
            .as_object_mut()
            .expect("JWK object")
            .insert(field.into(), value);
        assert!(
            matches!(
                DatabaseTokenVerifier::from_jwks(&jwks_with(vec![invalid]), policy()),
                Err(JwtValidationError::InvalidJwks)
            ),
            "field {field} must fail closed"
        );
    }
}

#[test]
fn token_header_rejects_missing_kid_embedded_keys_urls_and_custom_fields() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let claims = valid_claims();
    let valid = sign_payload(claims.clone(), KID, &key);

    for (field, value) in [
        ("jku", json!("https://attacker.example.test/jwks.json")),
        ("jwk", jwk_value(&key, KID)),
        ("unexpected", json!(true)),
    ] {
        let invalid = mutate_header(&valid, |header| {
            header.insert(field.into(), value);
        });
        assert_eq!(
            verifier.verify(&invalid, claims.profile.database_role(), NOW + 1),
            Err(JwtValidationError::InvalidHeader)
        );
    }

    let missing_kid = mutate_header(&valid, |header| {
        header.remove("kid");
    });
    assert_eq!(
        verifier.verify(&missing_kid, claims.profile.database_role(), NOW + 1,),
        Err(JwtValidationError::InvalidHeader)
    );
}

#[test]
fn token_rejects_unknown_kid_wrong_algorithm_and_tampered_signature() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let claims = valid_claims();

    let unknown_kid = sign_payload(claims.clone(), "unknown-kid", &key);
    assert_eq!(
        verifier.verify(&unknown_kid, claims.profile.database_role(), NOW + 1),
        Err(JwtValidationError::UnknownKeyId)
    );

    let valid = sign_payload(claims.clone(), KID, &key);
    let wrong_algorithm = mutate_header(&valid, |header| {
        header.insert("alg".into(), json!("RS256"));
    });
    assert_eq!(
        verifier.verify(&wrong_algorithm, claims.profile.database_role(), NOW + 1,),
        Err(JwtValidationError::InvalidHeader)
    );

    let mut segments = valid.split('.').map(str::to_owned).collect::<Vec<_>>();
    let replacement = if segments[2].starts_with('A') {
        "B"
    } else {
        "A"
    };
    segments[2].replace_range(..1, replacement);
    assert_eq!(
        verifier.verify(
            &segments.join("."),
            claims.profile.database_role(),
            NOW + 1,
        ),
        Err(JwtValidationError::InvalidSignature)
    );
}

#[test]
fn claims_reject_wrong_resource_time_and_requested_role() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_claims();
    let expected_role = base.profile.database_role();

    let mut invalid_claims = Vec::new();

    let mut wrong_issuer = base.clone();
    wrong_issuer.issuer = "https://attacker.example.test".into();
    invalid_claims.push(wrong_issuer);

    let mut wrong_audience = base.clone();
    wrong_audience.audience = "https://candidate.example.test/resources/mcp".into();
    invalid_claims.push(wrong_audience);

    let mut wrong_scope = base.clone();
    wrong_scope.scope = "mcp".into();
    invalid_claims.push(wrong_scope);

    let mut future_iat = base.clone();
    future_iat.issued_at = NOW + 1;
    future_iat.expires_at = NOW + 121;
    invalid_claims.push(future_iat);

    let mut expired = base.clone();
    expired.issued_at = NOW - 121;
    expired.expires_at = NOW - 1;
    invalid_claims.push(expired);

    let mut ttl_too_long = base.clone();
    ttl_too_long.expires_at = NOW + 301;
    invalid_claims.push(ttl_too_long);

    let mut ttl_too_short = base.clone();
    ttl_too_short.expires_at = NOW + 29;
    invalid_claims.push(ttl_too_short);

    for claims in invalid_claims {
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, expected_role, NOW),
            Err(JwtValidationError::InvalidClaims)
        );
    }

    let token = sign_payload(base.clone(), KID, &key);
    assert_eq!(
        verifier.verify(&token, DatabaseProfile::BusinessAdmin.database_role(), NOW),
        Err(JwtValidationError::RequestedRoleMismatch)
    );
}

#[test]
fn claims_schema_rejects_missing_unknown_and_illegal_identity_fields() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_claims();
    let expected_role = base.profile.database_role();

    let mut missing_claim = serde_json::to_value(base.clone()).expect("claims JSON");
    missing_claim
        .as_object_mut()
        .expect("claims object")
        .remove("profile");
    let token = sign_payload(missing_claim, KID, &key);
    assert_eq!(
        verifier.verify(&token, expected_role, NOW),
        Err(JwtValidationError::InvalidToken)
    );

    let mut unknown_claim = serde_json::to_value(base.clone()).expect("claims JSON");
    unknown_claim
        .as_object_mut()
        .expect("claims object")
        .insert("unexpected".into(), json!(true));
    let token = sign_payload(unknown_claim, KID, &key);
    assert_eq!(
        verifier.verify(&token, expected_role, NOW),
        Err(JwtValidationError::InvalidToken)
    );

    let too_long = "a".repeat(65);
    for subject in ["contains:delimiter", "含有非ASCII", "", too_long.as_str()] {
        let mut claims = base.clone();
        claims.subject = subject.into();
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, expected_role, NOW),
            Err(JwtValidationError::InvalidIdentity)
        );
    }
}

#[test]
fn identity_codec_rejects_ambiguity_unknown_versions_and_oversize_values() {
    let identity = ordinary_identity();
    let encoded = identity.encode_authn_id().expect("encode identity");

    assert_eq!(
        encoded,
        "candidate.example.test:v1;u=usr_01J00000000000000000000000;p=ordinary"
    );
    assert_eq!(decode_authn_id(&encoded), Ok(identity.clone()));
    let system_user = format!("oauth:{encoded}");
    assert_eq!(decode_system_user(&system_user), Ok(identity));

    let wrong_version = "candidate.example.test:v2;u=usr_01J00000000000000000000000;p=ordinary";
    assert!(decode_authn_id(wrong_version).is_err());
    assert!(decode_authn_id(&encoded.replacen("p=ordinary", "p=business-admin", 1)).is_err());
    assert!(decode_system_user(&format!("scram:{encoded}")).is_err());
    assert!(decode_authn_id(&"x".repeat(MAX_AUTHN_ID_BYTES + 1)).is_err());
    assert!(decode_system_user(&format!("oauth:{}", "x".repeat(MAX_AUTHN_ID_BYTES + 1))).is_err());
    assert!(decode_system_user(
        "oauth:pggomtm:v2;u=usr;actor=client:cli;d=dlg;m=oauth;a=7;p=ordinary"
    )
    .is_err());
}

#[test]
fn profile_role_mapping_is_closed_and_non_inheriting() {
    let mappings = BTreeMap::from([
        (DatabaseProfile::Ordinary, "ordinary"),
        (DatabaseProfile::BusinessAdmin, "business_admin"),
        (DatabaseProfile::DatabaseDeveloper, "database_developer"),
    ]);

    for (profile, canonical_name) in mappings {
        assert_eq!(profile.database_role(), canonical_name);
        assert_eq!(
            serde_json::to_value(profile).expect("serialize profile"),
            json!(canonical_name)
        );
        let identity = AuthenticatedIdentity {
            user_id: "usr_profile_mapping".into(),
            profile,
            issuer_host: ISSUER_HOST.into(),
        };
        assert!(
            identity
                .encode_authn_id()
                .expect("encode profile identity")
                .ends_with(&format!(";p={canonical_name}"))
        );
    }
}

#[test]
fn hyphenated_profiles_and_prefixed_roles_fail_closed() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_claims();

    for profile in ["business-admin", "database-developer"] {
        let mut claims = serde_json::to_value(base.clone()).expect("claims JSON");
        claims
            .as_object_mut()
            .expect("claims object")
            .insert("profile".into(), json!(profile));
        let token = sign_payload(claims, KID, &key);
        assert_eq!(
            verifier.verify(&token, profile, NOW),
            Err(JwtValidationError::InvalidToken),
            "hyphenated profile {profile} must not enter the v1 contract"
        );
    }

    let ordinary_token = sign_payload(base.clone(), KID, &key);
    for role in [
        "gomtm_candidate_ordinary",
        "gomtm_ordinary",
        "gomtm_candidate_business_admin",
        "gomtm_platform_admin",
    ] {
        assert_eq!(
            verifier.verify(&ordinary_token, role, NOW),
            Err(JwtValidationError::RequestedRoleMismatch),
            "prefixed role {role} must not override the signed profile"
        );
    }
}

#[test]
fn requested_service_migration_cluster_and_unknown_roles_fail_closed() {
    let key = signing_key();
    let verifier = verifier(&jwks_with(vec![jwk_value(&key, KID)]));
    let base = valid_claims();
    let ordinary_token = sign_payload(base, KID, &key);

    for forbidden_role in [
        DatabaseProfile::BusinessAdmin.database_role(),
        DatabaseProfile::DatabaseDeveloper.database_role(),
        "gomtm_test_auth_runtime",
        "gomtm_test_migration_owner",
        "gomtm_platform_admin",
        "gomtm_candidate_unknown",
        "service",
        "migration",
        "cluster-admin",
    ] {
        assert_eq!(
            verifier.verify(&ordinary_token, forbidden_role, NOW),
            Err(JwtValidationError::RequestedRoleMismatch),
            "requested role {forbidden_role} must not override the signed profile"
        );
    }
}
