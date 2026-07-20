use mtmpg_executor::auth::{
    AUTH_WINDOW_SECONDS, AuthenticationError, EXECUTE_PATH, HmacAuthenticator, SignedRequest,
    WIRE_VERSION,
};

const NOW: i64 = 1_800_000_000;
const NONCE: &str = "00112233445566778899aabbccddeeff";
const BODY: &[u8] = br#"{"principal":{"user_id":"usr_01"}}"#;
const SIGNATURE: &str = "51ea06b86989ea11d634b6a4c25f00da8e1ddd50d21e4b6da9a77721b0da8d25";

fn authenticator() -> HmacAuthenticator {
    HmacAuthenticator::new(vec![0x0b; 32], 16).expect("valid test HMAC configuration")
}

fn request<'a>(
    method: &'a str,
    path: &'a str,
    version: &'a str,
    timestamp: i64,
    nonce: &'a str,
    body: &'a [u8],
    signature: &'a str,
) -> SignedRequest<'a> {
    SignedRequest {
        method,
        path,
        version,
        timestamp,
        nonce,
        body,
        signature,
    }
}

#[test]
fn accepts_the_fixed_canonical_hmac_vector_once() {
    let authenticator = authenticator();
    let signed = request(
        "POST",
        EXECUTE_PATH,
        WIRE_VERSION,
        NOW,
        NONCE,
        BODY,
        SIGNATURE,
    );

    assert_eq!(authenticator.verify(&signed, NOW), Ok(()));
    assert_eq!(
        authenticator.verify(&signed, NOW),
        Err(AuthenticationError::Unauthorized)
    );
}

#[test]
fn timestamp_window_is_inclusive_and_fails_closed_outside_it() {
    let at_past_boundary = authenticator();
    let signed = request(
        "POST",
        EXECUTE_PATH,
        WIRE_VERSION,
        NOW,
        NONCE,
        BODY,
        SIGNATURE,
    );
    assert_eq!(
        at_past_boundary.verify(&signed, NOW + AUTH_WINDOW_SECONDS),
        Ok(())
    );

    let outside_past_boundary = authenticator();
    assert_eq!(
        outside_past_boundary.verify(&signed, NOW + AUTH_WINDOW_SECONDS + 1),
        Err(AuthenticationError::Unauthorized)
    );

    let at_future_boundary = authenticator();
    assert_eq!(
        at_future_boundary.verify(&signed, NOW - AUTH_WINDOW_SECONDS),
        Ok(())
    );

    let outside_future_boundary = authenticator();
    assert_eq!(
        outside_future_boundary.verify(&signed, NOW - AUTH_WINDOW_SECONDS - 1),
        Err(AuthenticationError::Unauthorized)
    );
}

#[test]
fn every_authenticated_component_is_covered_by_the_signature() {
    let mutations = [
        request(
            "GET",
            EXECUTE_PATH,
            WIRE_VERSION,
            NOW,
            NONCE,
            BODY,
            SIGNATURE,
        ),
        request(
            "POST",
            "/v1/other",
            WIRE_VERSION,
            NOW,
            NONCE,
            BODY,
            SIGNATURE,
        ),
        request("POST", EXECUTE_PATH, "v2", NOW, NONCE, BODY, SIGNATURE),
        request(
            "POST",
            EXECUTE_PATH,
            WIRE_VERSION,
            NOW + 1,
            NONCE,
            BODY,
            SIGNATURE,
        ),
        request(
            "POST",
            EXECUTE_PATH,
            WIRE_VERSION,
            NOW,
            "ffeeddccbbaa99887766554433221100",
            BODY,
            SIGNATURE,
        ),
        request(
            "POST",
            EXECUTE_PATH,
            WIRE_VERSION,
            NOW,
            NONCE,
            br#"{"principal":{"user_id":"usr_02"}}"#,
            SIGNATURE,
        ),
    ];

    for mutation in mutations {
        assert_eq!(
            authenticator().verify(&mutation, NOW),
            Err(AuthenticationError::Unauthorized)
        );
    }
}

#[test]
fn malformed_nonce_and_signature_share_the_unauthorized_result() {
    for (nonce, signature) in [
        ("001122", SIGNATURE),
        ("00112233445566778899AABBCCDDEEFF", SIGNATURE),
        (NONCE, "not-hex"),
        (NONCE, "00"),
    ] {
        let signed = request(
            "POST",
            EXECUTE_PATH,
            WIRE_VERSION,
            NOW,
            nonce,
            BODY,
            signature,
        );
        assert_eq!(
            authenticator().verify(&signed, NOW),
            Err(AuthenticationError::Unauthorized)
        );
    }
}

#[test]
fn a_full_replay_store_fails_closed_without_evicting_live_nonces() {
    let authenticator =
        HmacAuthenticator::new(vec![0x0b; 32], 1).expect("valid bounded replay configuration");
    let first = request(
        "POST",
        EXECUTE_PATH,
        WIRE_VERSION,
        NOW,
        NONCE,
        BODY,
        SIGNATURE,
    );
    assert_eq!(authenticator.verify(&first, NOW), Ok(()));

    let second = request(
        "POST",
        EXECUTE_PATH,
        WIRE_VERSION,
        NOW,
        "ffeeddccbbaa99887766554433221100",
        BODY,
        "5e906b0b3a901b21f03f9e62d326e08a6f3114e70452dc465cd1bdd559a150b8",
    );
    assert_eq!(
        authenticator.verify(&second, NOW),
        Err(AuthenticationError::Unauthorized)
    );
    assert_eq!(
        authenticator.verify(&first, NOW),
        Err(AuthenticationError::Unauthorized)
    );
}
