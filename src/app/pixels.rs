//! A pixel board everyone paints on together.

use topcoat::{
    Result,
    context::{Cx, context},
    metadata::Metadata,
    router::page,
    runtime::{Event, Signal, connected, procedure, shard, signal},
    view::{View, emit, live, view},
};

use crate::features::board::{Board, PALETTE, WIDTH};

#[page(with(Metadata::new().title("Pixel Board")))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    // The selected color lives in the browser. Picking one never asks the server.
    let color = signal(cx, || 2u8);

    Ok(view! {
        <main class="pixels min-h-screen">
            <div class="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-8">
                <header class="flex items-end justify-between">
                    <div>
                        <p class="kicker">"Topcoat 0.12 · live!"</p>
                        <h1 class="title">"Pixel Board"</h1>
                    </div>
                    stats()
                </header>

                <div class="flex flex-wrap items-center gap-2">
                    for (i, hex) in PALETTE.iter().enumerate().skip(1) {
                        let i = i as u8;
                        <button
                            class="swatch"
                            style=(format!("background:{hex}"))
                            :class=$(if color.get() == i {
                                "swatch on"
                            } else {
                                "swatch"
                            })
                            @click=$(|_e: Event| color.set(i))
                            aria-label=(format!("Color {i}"))
                        ></button>
                    }
                    <button
                        :class=$(if color.get() == 0u8 { "eraser on" } else { "eraser" })
                        @click=$(|_e: Event| color.set(0u8))
                    >
                        "Eraser"
                    </button>
                </div>

                canvas(color: color)
            </div>
        </main>
    })
}

/// Who is here and how much has been painted. Updates with the board.
#[shard]
async fn stats(cx: &Cx) -> Result<impl View> {
    Ok(live! {
        let board = context::<Board>(cx);
        let mut changed = board.subscribe();
        loop {
            let token = emit! {
                <div class="flex gap-6 text-right">
                    <div>
                        <p class="stat">(board.online())</p>
                        <p class="stat-label">"online"</p>
                    </div>
                    <div>
                        <p class="stat">(board.paints())</p>
                        <p class="stat-label">"pixels painted"</p>
                    </div>
                </div>
            }?;
            if !connected(cx) {
                break Ok(token);
            }
            changed.recv().await.ok();
        }
    })
}

/// The board itself. Every open page renders it again after each paint.
#[shard]
async fn canvas(cx: &Cx, color: Signal<u8>) -> Result<impl View> {
    Ok(live! {
        let board = context::<Board>(cx);
        let mut changed = board.subscribe();
        // Counted as online from the moment the WebSocket is open.
        let mut presence = None;
        loop {
            let pixels = board.pixels();
            // One handler for the whole board. Each cell's value is its index, so the
            // page ships one function instead of 960.
            let token = emit! {
                <div
                    class="board"
                    style=(format!("grid-template-columns:repeat({WIDTH},1fr)"))
                    @pointerdown=$(async |e: Event| {
                        paint(e.target.value, color.get()).await;
                    })
                    @pointerover=$(async |e: Event| {
                        if e.buttons == 1.0 {
                            paint(e.target.value, color.get()).await;
                        }
                    })
                >
                    for (i, p) in pixels.into_iter().enumerate() {
                        <button class=(format!("cell c{p}")) value=(i)></button>
                    }
                </div>
            }?;
            if !connected(cx) {
                break Ok(token);
            }
            if presence.is_none() {
                presence = Some(board.join());
            }
            changed.recv().await.ok();
        }
    })
}

/// Paints the cell with this index. Anything else that was clicked is ignored.
#[procedure]
pub async fn paint(cx: &Cx, cell: String, color: u8) -> Result<bool> {
    let Ok(i) = cell.parse::<usize>() else {
        return Ok(false);
    };
    Ok(context::<Board>(cx).paint(i % WIDTH, i / WIDTH, color))
}
