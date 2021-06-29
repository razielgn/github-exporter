use anyhow::Result;
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, Encoder, HistogramVec, IntCounterVec,
    TextEncoder,
};
use std::net::SocketAddr;
use tracing::{info, span, Instrument, Level};

pub async fn listen(addr: &SocketAddr) -> Result<()> {
    let make_service = make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(handle)) });
    let server = Server::bind(addr).serve(make_service);

    info!("listening on {}", addr);

    server.await?;

    Ok(())
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let span = span!(
        Level::INFO,
        "request",
        method = ?req.method(),
        uri = ?req.uri(),
        headers = ?req.headers()
    );

    let timer = HTTP_REQ_HISTOGRAM
        .with_label_values(&[req.uri().path()])
        .start_timer();

    async move {
        let mut response = Response::new(Body::empty());

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/healthz") => {
                *response.body_mut() = Body::from("OK");
            }
            (&Method::GET, "/metrics") => {
                let mut buf = Vec::with_capacity(100_000);
                let encoder = TextEncoder::new();
                let metric_families = prometheus::gather();
                encoder.encode(&metric_families, &mut buf).unwrap();

                response
                    .headers_mut()
                    .append(CONTENT_TYPE, encoder.format_type().parse().unwrap());
                *response.body_mut() = Body::from(buf);
            }
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }

        HTTP_COUNTER
            .with_label_values(&[response.status().as_str(), req.uri().path()])
            .inc();
        timer.observe_duration();

        info!(status = ?(&response.status()), "response");

        Ok(response)
    }
    .instrument(span)
    .await
}

lazy_static! {
    pub static ref HTTP_COUNTER: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Number of HTTP requests made.",
        &["status_code", "path"]
    )
    .unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["path"]
    )
    .unwrap();
}
