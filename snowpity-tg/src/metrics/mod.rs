mod macros;

use crate::util::prelude::*;
use crate::{err_ctx, HttpServerError, Result};
use futures::prelude::*;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use prometheus::Encoder;
use std::convert::Infallible;
use std::net::SocketAddr;

pub(crate) use macros::*;

pub(crate) async fn run_metrics(abort: impl Future<Output = ()>) -> Result {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let make_svc = make_service_fn(|conn: &AddrStream| {
        let remote_addr = conn.remote_addr();
        let local_addr = conn.local_addr();

        future::ok::<_, Infallible>(service_fn(move |request| {
            handle_metrics(request).instrument(info_span!(
                "incomming_connection",
                %remote_addr,
                %local_addr
            ))
        }))
    });

    Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(abort)
        .await
        .map_err(err_ctx!(HttpServerError::Serve))
}

#[instrument(skip_all, fields(
    method = %request.method(),
    uri = %request.uri(),
))]
async fn handle_metrics(request: Request<Body>) -> Result<Response<Body>, Infallible> {
    trace!("Received an HTTP request");

    Ok(match (request.method(), request.uri().path()) {
        (&hyper::Method::GET, "/metrics") => {
            let metrics = prometheus::gather();
            let mut buffer = vec![];
            prometheus::TextEncoder::new()
                .encode(&metrics, &mut buffer)
                .expect("BUG: couldn't encode metrics");

            Response::new(buffer.into())
        }
        _ => {
            error!(
                request = format_args!("{request:#?}"),
                "Received an unexpected request",
            );
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_FOUND;
            response
        }
    })
}
