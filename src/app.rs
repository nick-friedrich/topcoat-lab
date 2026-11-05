mod mission;
mod wall;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, fontsource::fontsource_font},
    router::{RouterBuilder, RouterBuilderDiscoverExt, Slot, href, layout, module_router, page},
    runtime::RouterBuilderRuntimeExt,
    tailwind,
    view::{View, view},
};

/// Geist for text, Geist Mono for code and telemetry. Loaded from Fontsource.
const GEIST: Font = fontsource_font!(GEIST, weight: [400, 700], style: Normal);
const GEIST_MONO: Font = fontsource_font!(GEIST_MONO, weight: [400, 700], style: Normal);

/// Builds the application's router.
pub fn router() -> RouterBuilder {
    // Pages come from the module tree: `app::wall` is served at `/wall`.
    module_router!()
        // Shards and procedures have no place in the module tree.
        .discover()
        .assets(AssetBundle::load().unwrap())
        // Lets the browser runtime rerun pages; register it after other layers.
        .runtime()
}

#[layout]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Topcoat Lab"</title>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                topcoat::font::link(font: GEIST)
                topcoat::font::link(font: GEIST_MONO)
                topcoat::runtime::script()
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <main
            class="grid min-h-screen place-items-center bg-[#120c08] px-6 text-[#f3e6d4]"
        >
            <div class="w-full max-w-3xl">
                <p class="font-mono text-sm tracking-widest text-[#e8743b] uppercase">
                    "topcoat-lab"
                </p>
                <h1 class="mt-2 text-5xl font-bold tracking-tight">
                    "Two apps. One Rust binary."
                </h1>
                <div class="mt-10 grid gap-6 sm:grid-cols-2">
                    <a
                        class="group rounded-2xl border border-[#3a2a1e] bg-[#f6efe3] p-6 text-[#2b1d14] transition hover:-translate-y-1"
                        href=(href!(wall::page))
                    >
                        <p class="text-3xl">"🦀"</p>
                        <h2 class="mt-3 text-2xl font-bold">"It Compiles."</h2>
                        <p class="mt-1 text-[#6b5444]">
                            "A guestbook that only accepts Rust that rustc accepts."
                        </p>
                    </a>
                    <a
                        class="group rounded-2xl border border-[#1d3b2a] bg-[#050807] p-6 font-mono text-[#5cff9d] transition hover:-translate-y-1"
                        href=(href!(mission::page))
                    >
                        <p class="text-3xl">"◉"</p>
                        <h2 class="mt-3 text-2xl font-bold">"MISSION CONTROL"</h2>
                        <p class="mt-1 text-[#3f9e66]">
                            "Live telemetry from this Mac, streamed every second."
                        </p>
                    </a>
                </div>
            </div>
        </main>
    })
}
