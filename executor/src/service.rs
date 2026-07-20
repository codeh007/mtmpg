use std::env;
use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_server::tls_rustls::RustlsConfig;
use p256::ecdsa::SigningKey;
use p256::pkcs8::DecodePrivateKey;
use serde::Serialize;
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

use crate::auth::{HmacAuthenticator, SignedRequest};
use crate::issuer::{DatabaseTokenIssuer, IssuerConfig, IssuerError};
use crate::libpq::{
    Cancellation, DatabaseConfig, DatabaseError, DatabaseErrorKind, ExecutionResult,
    install_auth_data_hook,
};
use crate::protocol::{MAX_REQUEST_BODY_BYTES, ProtocolError, parse_execute_request};
use crate::token_registry::ConnectionTokenRegistry;

const MAX_CONCURRENCY: usize = 32;
const REPLAY_CAPACITY: usize = 4_096;

#[derive(Clone)]
struct AppState {
    authenticator: Arc<HmacAuthenticator>,
    issuer: Arc<DatabaseTokenIssuer>,
    database: DatabaseConfig,
    registry: Arc<ConnectionTokenRegistry>,
    concurrency: Arc<Semaphore>,
}

struct RuntimeConfig {
    state: AppState,
    listen: SocketAddr,
    tls_cert_path: String,
    tls_key_path: String,
}

impl RuntimeConfig {
    fn from_environment() -> Result<Self, Box<dyn Error>> {
        let hmac_secret = Zeroizing::new(fs::read(required_environment(
            "MTMPG_EXECUTOR_HMAC_SECRET_PATH",
        )?)?);
        let authenticator = HmacAuthenticator::new(hmac_secret.to_vec(), REPLAY_CAPACITY)
            .map_err(|_| invalid_input("invalid HMAC configuration"))?;

        let signing_pem = Zeroizing::new(fs::read_to_string(required_environment(
            "MTMPG_EXECUTOR_SIGNING_KEY_PATH",
        )?)?);
        let signing_key = SigningKey::from_pkcs8_pem(&signing_pem)
            .map_err(|_| invalid_input("invalid signing key"))?;
        let issuer_config = IssuerConfig::new(
            required_environment("MTMPG_EXECUTOR_ISSUER")?,
            required_environment("MTMPG_EXECUTOR_AUDIENCE")?,
            required_environment("MTMPG_EXECUTOR_KEY_ID")?,
        )
        .map_err(|_| invalid_input("invalid issuer configuration"))?;

        let registry = Arc::new(
            ConnectionTokenRegistry::with_capacity(MAX_CONCURRENCY)
                .map_err(|_| invalid_input("invalid token registry configuration"))?,
        );
        install_auth_data_hook(Arc::clone(&registry))
            .map_err(|_| invalid_input("libpq auth hook installation failed"))?;

        Ok(Self {
            state: AppState {
                authenticator: Arc::new(authenticator),
                issuer: Arc::new(DatabaseTokenIssuer::new(issuer_config, signing_key)),
                database: DatabaseConfig::canonical(required_environment(
                    "MTMPG_EXECUTOR_POSTGRES_CA_PATH",
                )?),
                registry,
                concurrency: Arc::new(Semaphore::new(MAX_CONCURRENCY)),
            },
            listen: required_environment("MTMPG_EXECUTOR_LISTEN")?
                .parse()
                .map_err(|_| invalid_input("invalid listen address"))?,
            tls_cert_path: required_environment("MTMPG_EXECUTOR_TLS_CERT_PATH")?,
            tls_key_path: required_environment("MTMPG_EXECUTOR_TLS_KEY_PATH")?,
        })
    }
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let config = RuntimeConfig::from_environment()?;
    let tls = RustlsConfig::from_pem_file(&config.tls_cert_path, &config.tls_key_path).await?;
    let application = Router::new()
        .route("/ready", get(ready))
        .route(crate::auth::EXECUTE_PATH, post(execute))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(config.state);
    axum_server::bind_rustls(config.listen, tls)
        .serve(application.into_make_service())
        .await?;
    Ok(())
}

async fn ready() -> StatusCode {
    StatusCode::OK
}

async fn execute(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let envelope = match parse_envelope(&headers, &body) {
        Ok(envelope) => envelope,
        Err(()) => return error_response(StatusCode::UNAUTHORIZED, "unauthorized", None),
    };
    let now = match unix_time() {
        Ok(now) => now,
        Err(()) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "unavailable", None),
    };
    if state.authenticator.verify(&envelope, now) != Ok(()) {
        return error_response(StatusCode::UNAUTHORIZED, "unauthorized", None);
    }
    let request = match parse_execute_request(&body) {
        Ok(request) => request,
        Err(error) => return protocol_error(error),
    };
    let correlation_id = request.correlation_id.clone();
    let token = match state.issuer.issue(&request.principal, now) {
        Ok(token) => token,
        Err(error) => return issuer_error(error, &correlation_id),
    };
    let permit = match Arc::clone(&state.concurrency).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "busy",
                Some(&correlation_id),
            );
        }
    };

    let database = state.database.clone();
    let registry = Arc::clone(&state.registry);
    let cancellation = Cancellation::new();
    let worker_cancellation = cancellation.clone();
    let mut cancel_on_drop = CancelOnDrop(Some(cancellation));
    let worker = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        crate::libpq::execute(
            &database,
            registry,
            &request,
            token.into_secret(),
            &worker_cancellation,
        )
    });
    let result = worker.await;
    cancel_on_drop.0 = None;
    match result {
        Ok(Ok(result)) => success_response(result),
        Ok(Err(error)) => database_error(error, &correlation_id),
        Err(_) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            Some(&correlation_id),
        ),
    }
}

fn parse_envelope<'a>(headers: &'a HeaderMap, body: &'a [u8]) -> Result<SignedRequest<'a>, ()> {
    let version = exactly_one_header(headers, "x-executor-version")?;
    let timestamp = exactly_one_header(headers, "x-executor-timestamp")?
        .parse()
        .map_err(|_| ())?;
    let nonce = exactly_one_header(headers, "x-executor-nonce")?;
    let signature = exactly_one_header(headers, "x-executor-signature")?;
    Ok(SignedRequest {
        method: "POST",
        path: crate::auth::EXECUTE_PATH,
        version,
        timestamp,
        nonce,
        body,
        signature,
    })
}

fn exactly_one_header<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<&'a str, ()> {
    let mut values = headers.get_all(name).iter();
    let value = values.next().ok_or(())?;
    if values.next().is_some() {
        return Err(());
    }
    value.to_str().map_err(|_| ())
}

fn protocol_error(error: ProtocolError) -> Response {
    match error {
        ProtocolError::LimitExceeded => {
            error_response(StatusCode::PAYLOAD_TOO_LARGE, "budget_exceeded", None)
        }
        ProtocolError::InvalidRequest | ProtocolError::ConfirmationRequired => {
            error_response(StatusCode::BAD_REQUEST, "invalid_request", None)
        }
    }
}

fn issuer_error(error: IssuerError, correlation_id: &str) -> Response {
    let (status, category) = match error {
        IssuerError::CredentialExpiresTooSoon => (StatusCode::UNAUTHORIZED, "unauthorized"),
        IssuerError::InvalidPrincipal => (StatusCode::BAD_REQUEST, "invalid_request"),
        IssuerError::InvalidConfiguration | IssuerError::SigningFailed => {
            (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
        }
    };
    error_response(status, category, Some(correlation_id))
}

fn database_error(error: DatabaseError, correlation_id: &str) -> Response {
    let (status, category) = match error.kind {
        DatabaseErrorKind::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
        DatabaseErrorKind::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        DatabaseErrorKind::Rejected => (StatusCode::UNPROCESSABLE_ENTITY, "database_rejected"),
        DatabaseErrorKind::BudgetExceeded => (StatusCode::PAYLOAD_TOO_LARGE, "budget_exceeded"),
        DatabaseErrorKind::DeadlineExceeded => (StatusCode::GATEWAY_TIMEOUT, "deadline_exceeded"),
    };
    let error = ErrorBody {
        category,
        message: "request could not be completed",
        correlation_id: Some(correlation_id),
        sqlstate_class: error.sqlstate_class.as_deref(),
    };
    (status, Json(ErrorEnvelope { error })).into_response()
}

fn success_response(result: ExecutionResult) -> Response {
    (StatusCode::OK, Json(SuccessEnvelope { result })).into_response()
}

fn error_response(
    status: StatusCode,
    category: &'static str,
    correlation_id: Option<&str>,
) -> Response {
    let error = ErrorBody {
        category,
        message: "request could not be completed",
        correlation_id,
        sqlstate_class: None,
    };
    (status, Json(ErrorEnvelope { error })).into_response()
}

#[derive(Serialize)]
struct SuccessEnvelope {
    result: ExecutionResult,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    category: &'a str,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sqlstate_class: Option<&'a str>,
}

struct CancelOnDrop(Option<Cancellation>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(cancellation) = &self.0 {
            cancellation.cancel();
        }
    }
}

fn required_environment(name: &str) -> Result<String, IoError> {
    env::var(name).map_err(|_| {
        IoError::new(
            ErrorKind::InvalidInput,
            format!("required configuration is unavailable: {name}"),
        )
    })
}

fn unix_time() -> Result<i64, ()> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?
        .as_secs();
    i64::try_from(seconds).map_err(|_| ())
}

fn invalid_input(message: &'static str) -> IoError {
    IoError::new(ErrorKind::InvalidInput, message)
}
