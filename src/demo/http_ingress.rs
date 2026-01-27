use std::{convert::Infallible, net::SocketAddr, sync::Arc, thread};

use anyhow::{Context, Result};
use hyper::{
    Body, Method, Request, Response, Server, StatusCode,
    body::to_bytes,
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
};
use serde_json::json;
use tokio::{runtime::Runtime, sync::oneshot};

use crate::demo::runner_host::{DemoRunnerHost, FlowOutcome, OperatorContext};
use crate::domains::Domain;
use crate::operator_log;

#[derive(Clone)]
pub struct HttpIngressConfig {
    pub bind_addr: SocketAddr,
    pub domains: Vec<Domain>,
    pub runner_host: Arc<DemoRunnerHost>,
}

pub struct HttpIngressServer {
    shutdown: Option<oneshot::Sender<()>>,
    handle: Option<thread::JoinHandle<Result<()>>>,
}

impl HttpIngressServer {
    pub fn start(config: HttpIngressConfig) -> Result<Self> {
        let state = Arc::new(HttpIngressState {
            runner_host: config.runner_host,
            domains: config.domains,
        });
        let (tx, rx) = oneshot::channel();
        let addr = config.bind_addr;
        let handle = thread::Builder::new()
            .name("demo-ingress".to_string())
            .spawn(move || -> Result<()> {
                let runtime = Runtime::new().context("failed to create ingress runtime")?;
                runtime.block_on(async move {
                    let service = make_service_fn(move |_| {
                        let state = state.clone();
                        async move {
                            Ok::<_, Infallible>(service_fn(move |req| {
                                handle_request(req, state.clone())
                            }))
                        }
                    });
                    operator_log::info(
                        module_path!(),
                        format!("demo ingress listening on http://{}", addr),
                    );
                    let server = Server::try_bind(&addr)?.serve(service);
                    server
                        .with_graceful_shutdown(async move {
                            let _ = rx.await;
                        })
                        .await?;
                    Ok(())
                })
            })?;
        Ok(Self {
            shutdown: Some(tx),
            handle: Some(handle),
        })
    }

    pub fn stop(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let joined = handle
                .join()
                .map_err(|err| anyhow::anyhow!("ingress server panicked: {err:?}"))?;
            joined?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct HttpIngressState {
    runner_host: Arc<DemoRunnerHost>,
    domains: Vec<Domain>,
}

async fn handle_request(
    req: Request<Body>,
    state: Arc<HttpIngressState>,
) -> Result<Response<Body>, Infallible> {
    let response = match handle_request_inner(req, state).await {
        Ok(response) => response,
        Err(response) => response,
    };
    Ok(response)
}

async fn handle_request_inner(
    req: Request<Body>,
    state: Arc<HttpIngressState>,
) -> Result<Response<Body>, Response<Body>> {
    if req.method() != Method::POST && req.method() != Method::GET {
        return Err(error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "only GET/POST allowed",
        ));
    }
    let segments = req
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 4 || !segments[1].eq_ignore_ascii_case("ingress") {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "expected /{domain}/ingress/{provider}/{tenant}/{team?}",
        ));
    }
    let domain = match parse_domain(segments[0]) {
        Some(value) => value,
        None => return Err(error_response(StatusCode::NOT_FOUND, "unknown domain")),
    };
    if !state.domains.contains(&domain) {
        return Err(error_response(StatusCode::NOT_FOUND, "domain disabled"));
    }
    let provider = segments[2].to_string();
    let tenant = segments[3].to_string();
    let team = segments
        .get(4)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "default".to_string());
    let flow_id = if state
        .runner_host
        .supports_op(domain, &provider, "handle-webhook")
    {
        "handle-webhook".to_string()
    } else if state.runner_host.supports_op(domain, &provider, "ingest") {
        "ingest".to_string()
    } else {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "no ingress flow available",
        ));
    };
    let correlation_id = req
        .headers()
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let payload_bytes = to_bytes(req.into_body()).await.unwrap_or_default();
    let context = OperatorContext {
        tenant,
        team: Some(team),
        correlation_id,
    };
    let runner_host = state.runner_host.clone();
    let provider_for_exec = provider.clone();
    let flow_for_exec = flow_id.clone();
    let payload_for_exec = payload_bytes.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        runner_host.invoke_provider_op(
            domain,
            &provider_for_exec,
            &flow_for_exec,
            &payload_for_exec,
            &context,
        )
    })
    .await
    .map_err(|err| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("ingress invocation panicked: {err}"),
        )
    })?
    .map_err(|err| error_response(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let body = flow_outcome_body(&outcome);
    let status = if outcome.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_GATEWAY
    };
    Ok(json_response(status, body))
}

fn flow_outcome_body(outcome: &FlowOutcome) -> serde_json::Value {
    json!({
        "success": outcome.success,
        "mode": format!("{:?}", outcome.mode),
        "output": outcome.output,
        "raw": outcome.raw,
        "error": outcome.error,
    })
}

fn parse_domain(value: &str) -> Option<Domain> {
    match value.to_lowercase().as_str() {
        "messaging" => Some(Domain::Messaging),
        "events" => Some(Domain::Events),
        "secrets" => Some(Domain::Secrets),
        _ => None,
    }
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response<Body> {
    let body = json!({
        "success": false,
        "message": message.into()
    });
    json_response(status, body)
}

fn json_response(status: StatusCode, value: serde_json::Value) -> Response<Body> {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap_or_else(|err| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("failed to build response: {err}")))
                .unwrap()
        })
}
