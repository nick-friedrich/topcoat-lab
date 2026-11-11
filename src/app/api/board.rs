//! `/api/board`: read the board, or paint a pixel on it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, context},
    openapi::Operation,
    router::{StatusCode, content::Json, route},
};

use crate::features::board::{self, Board};

/// The whole board, row by row.
#[derive(Serialize, JsonSchema)]
struct Snapshot {
    width: usize,
    height: usize,
    /// One palette index per pixel, starting at the top left. 0 is empty.
    pixels: Vec<u8>,
    /// The colors, as CSS hex values, by palette index.
    palette: Vec<&'static str>,
}

/// One pixel to paint.
#[derive(Deserialize, JsonSchema)]
struct Paint {
    /// Column, from 0 on the left.
    x: usize,
    /// Row, from 0 at the top.
    y: usize,
    /// A palette index. 0 erases the pixel.
    color: u8,
}

#[route(
    GET,
    with(Operation::new()
        .summary("Read the board")
        .response_with::<Json<Snapshot>>(StatusCode::OK, "Every pixel on the board."))
)]
async fn read(cx: &Cx) -> Result<Json<Snapshot>> {
    Ok(Json(Snapshot {
        width: board::WIDTH,
        height: board::HEIGHT,
        pixels: context::<Board>(cx).pixels(),
        palette: board::PALETTE.to_vec(),
    }))
}

#[route(
    POST,
    with(Operation::new()
        .summary("Paint a pixel")
        .request::<Json<Paint>>()
        .response(StatusCode::NO_CONTENT, "Painted. Every open page updates.")
        .response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The pixel or color is out of range.",
        ))
)]
async fn paint(cx: &Cx, Json(input): Json<Paint>) -> Result<StatusCode> {
    Ok(
        if context::<Board>(cx).paint(input.x, input.y, input.color) {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        },
    )
}

#[route(
    DELETE,
    with(Operation::new()
        .summary("Clear the board")
        .response(StatusCode::NO_CONTENT, "Every pixel is empty again."))
)]
async fn clear(cx: &Cx) -> Result<StatusCode> {
    context::<Board>(cx).clear();
    Ok(StatusCode::NO_CONTENT)
}
