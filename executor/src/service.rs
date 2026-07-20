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
        let hmac_path = startup_stage(
            required_environment("MTMPG_EXECUTOR_HMAC_SECRET_PATH"),
            "hmac",
        )?;
        let hmac_secret = Zeroizing::new(startup_stage(fs::read(hmac_path), "hmac")?);
        let authenticator = startup_stage(
            HmacAuthenticator::new(hmac_secret.to_vec(), REPLAY_CAPACITY),
            "hmac",
        )?;

        let signing_key_path = startup_stage(
            required_environment("MTMPG_EXECUTOR_SIGNING_KEY_PATH"),
            "signing_key",
        )?;
        let signing_pem = Zeroizing::new(startup_stage(
            fs::read_to_string(signing_key_path),
            "signing_key",
        )?);
        let signing_key = startup_stage(SigningKey::from_pkcs8_pem(&signing_pem), "signing_key")?;
        let issuer = startup_stage(required_environment("MTMPG_EXECUTOR_ISSUER"), "issuer")?;
        let audience = startup_stage(required_environment("MTMPG_EXECUTOR_AUDIENCE"), "issuer")?;
        let key_id = startup_stage(required_environment("MTMPG_EXECUTOR_KEY_ID"), "issuer")?;
        let issuer_config = startup_stage(IssuerConfig::new(issuer, audience, key_id), "issuer")?;

        let registry = Arc::new(startup_stage(
            ConnectionTokenRegistry::with_capacity(MAX_CONCURRENCY),
            "token_registry",
        )?);
        startup_stage(install_auth_data_hook(Arc::clone(&registry)), "libpq")?;

        let ca_path = startup_stage(
            required_environment("MTMPG_EXECUTOR_POSTGRES_CA_PATH"),
            "database_tls",
        )?;
        let listen = startup_stage(
            startup_stage(required_environment("MTMPG_EXECUTOR_LISTEN"), "listen")?.parse(),
            "listen",
        )?;
        let tls_cert_path = startup_stage(
            required_environment("MTMPG_EXECUTOR_TLS_CERT_PATH"),
            "https_tls",
        )?;
        let tls_key_path = startup_stage(
            required_environment("MTMPG_EXECUTOR_TLS_KEY_PATH"),
            "https_tls",
        )?;

        Ok(Self {
            state: AppState {
                authenticator: Arc::new(authenticator),
                issuer: Arc::new(DatabaseTokenIssuer::new(issuer_config, signing_key)),
                database: DatabaseConfig::canonical(ca_path),
                registry,
                concurrency: Arc::new(Semaphore::new(MAX_CONCURRENCY)),
            },
            listen,
            tls_cert_path,
            tls_key_path,
        })
    }
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let config = RuntimeConfig::from_environment()?;
    let tls = startup_stage(
        RustlsConfig::from_pem_file(&config.tls_cert_path, &config.tls_key_path).await,
        "https_tls",
    )?;
    let application = Router::new()
        .route("/ready", get(ready))
        .route(crate::auth::EXECUTE_PATH, post(execute))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(config.state);
    startup_stage(
        axum_server::bind_rustls(config.listen, tls)
            .serve(application.into_make_service())
            .await,
        "https_server",
    )?;
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
    eprintln!("executor request failed: {}", error.stage.code());
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

fn startup_stage<T, E>(result: Result<T, E>, stage: &'static str) -> Result<T, Box<dyn Error>> {
    result.map_err(|_| {
        eprintln!("executor startup failed: {stage}");
        Box::new(invalid_input("executor startup failed")) as Box<dyn Error>
    })
}
