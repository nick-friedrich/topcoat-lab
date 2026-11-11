//! A JSON API for the pixel board, documented with OpenAPI.

mod board;

use topcoat::{
    Result,
    context::Cx,
    openapi::Operation,
    router::{Body, Next, layer, response::Response},
};

// Every route under `/api` inherits this tag in the OpenAPI document.
#[layer(with(Operation::new().tag("board")))]
async fn api(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    next.run(cx, body).await
}
