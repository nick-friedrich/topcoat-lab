//! "It Compiles": a guestbook where every message is a Rust program.

use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, Signal, procedure, shard, signal},
    view::{View, attributes, class, component, view},
};

use crate::components::button::button;
use crate::components::input::input;
use crate::features::{compiler, post::Post};

/// Every visitor starts with the most famous borrow-checker error.
const STARTER: &str = r#"fn main() {
    let greeting = String::from("Hello from the wall!");
    let moved = greeting;
    println!("{greeting}");
}
"#;

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    // Both live in the browser. Typing changes them without talking to the server.
    let author = signal(cx, String::new);
    let code = signal(cx, || STARTER.to_owned());
    // Bumped after a post, so the wall renders again.
    let revision = signal(cx, || 0u32);

    Ok(view! {
        <main class="wall min-h-screen bg-[#f6efe3] text-[#2b1d14]">
            <div class="mx-auto max-w-6xl px-6 py-12">
                <header class="mb-10">
                    <p class="font-mono text-sm tracking-widest text-[#b7410e] uppercase">"guestbook.rs"</p>
                    <h1 class="mt-2 text-6xl font-bold tracking-tight">"It Compiles."</h1>
                    <p class="mt-3 max-w-xl text-lg text-[#6b5444]">
                        "Leave a message. The catch: your message is a Rust program, and "
                        <code class="font-mono text-[#b7410e]">"rustc"</code>
                        " has to accept it."
                    </p>
                </header>

                <div class="grid gap-8 lg:grid-cols-[1.1fr_1fr]">
                    <section class="rounded-2xl border border-[#e3d5c0] bg-white/70 p-5">
                        <label class="text-sm font-medium" for="author">"Your name"</label>
                        input(attrs: attributes! {
                            id="author"
                            class="mt-1 mb-4 bg-white"
                            placeholder="Ferris"
                            maxlength="40"
                            @input=$(|e: Event| author.set(e.target.value))
                        })
                        <div class="editor rounded-xl bg-[#1d1611] p-4">
                            <textarea
                                class="block h-56 w-full resize-none bg-transparent font-mono text-[15px] leading-6 text-[#f3e6d4] outline-none"
                                spellcheck="false"
                                @input=$(|e: Event| code.set(e.target.value))
                            >(STARTER)</textarea>
                        </div>
                        checker(author: author, code: code, revision: revision.clone())
                    </section>

                    wall(revision: revision)
                </div>
            </div>
        </main>
    })
}

/// Asks rustc about the code. Runs on the server again every time the code changes.
#[shard]
async fn checker(
    author: Signal<String>,
    code: Signal<String>,
    revision: Signal<u32>,
) -> Result<impl View> {
    let verdict = compiler::check(&code.get()).await;
    let compiles = verdict.compiles();

    Ok(view! {
        <div class="mt-4 space-y-3">
            for diagnostic in &verdict.diagnostics {
                diagnostic_card(diagnostic: diagnostic)
            }

            <div class="flex items-center justify-between rounded-xl border border-[#e3d5c0] bg-white px-4 py-3">
                if compiles {
                    <p class="flex items-center gap-2 font-medium text-[#2f7d4f]">
                        <span class="crab-dance text-2xl">"🦀"</span>
                        "It compiles."
                    </p>
                } else {
                    <p class="font-medium text-[#b42318]">"rustc says no."</p>
                }
                <span class="font-mono text-xs text-[#8a7462]">"checked in " (verdict.millis) " ms"</span>
                button(
                    attrs: attributes! {
                        disabled=(!compiles)
                        class="bg-[#b7410e] text-white hover:bg-[#9a370c]"
                        @click=$(async |_e: Event| {
                            publish(author.get(), code.get()).await;
                            revision.increment();
                        })
                    },
                    "Post to the wall"
                )
            </div>
        </div>
    })
}

#[component]
async fn diagnostic_card(diagnostic: &compiler::Diagnostic) -> Result<impl View> {
    let error = diagnostic.level == "error";
    Ok(view! {
        <div class=(class!(
            "rounded-xl border-l-4 bg-[#1d1611] p-4 font-mono text-sm text-[#f3e6d4]",
            "border-[#f04438]" if error else "border-[#f5b83d]",
        ))>
            <p>
                <span class=(class!("font-bold", "text-[#ff6b5e]" if error else "text-[#f5b83d]"))>
                    (&diagnostic.level)
                    if let Some(code) = &diagnostic.code { "[" (code) "]" }
                </span>
                ": " (&diagnostic.message)
            </p>
            if let Some(line) = &diagnostic.line {
                <pre class="mt-2 text-[#c9b8a4]"><span class="text-[#7d6b5c]">(line.number) " | "</span>(&line.text)</pre>
                <pre class=(class!("text-[#ff6b5e]" if error else "text-[#f5b83d]"))>
                    (" ".repeat(line.number.to_string().len() + 3 + line.start))
                    ("^".repeat(line.end.saturating_sub(line.start).max(1)))
                    if let Some(label) = &line.label { " " (label) }
                </pre>
            }
            for note in &diagnostic.notes {
                <p class="mt-1 text-[#8fb8de]">"= " (note)</p>
            }
            if let Some(explanation) = &diagnostic.explanation {
                <p class="mt-2 font-sans text-[#c9b8a4] italic">(explanation)</p>
            }
        </div>
    })
}

/// The posts, newest first. Renders again whenever `revision` changes.
#[shard]
async fn wall(cx: &Cx, revision: Signal<u32>) -> Result<impl View> {
    let _ = revision.get();
    let posts = Post::recent(cx).await?;

    Ok(view! {
        <section class="space-y-4">
            <h2 class="text-sm font-medium tracking-widest text-[#8a7462] uppercase">
                (posts.len()) if posts.len() == 1 { " program on the wall" } else { " programs on the wall" }
            </h2>
            #[key(post.id)]
            for post in posts {
                <article id=(format!("post-{}", post.id)) class="post-in rounded-2xl border border-[#e3d5c0] bg-white p-4">
                    <div class="mb-2 flex items-baseline justify-between">
                        <p class="font-semibold">(&post.author)</p>
                        <p class="font-mono text-xs text-[#2f7d4f]">"✓ rustc " (post.millis) " ms"</p>
                    </div>
                    <pre class="overflow-x-auto rounded-lg bg-[#f6efe3] p-3 font-mono text-[13px] leading-5">(&post.code)</pre>
                </article>
            }
        </section>
    })
}

/// Saves a post. The browser could send anything, so compile it again here.
#[procedure]
pub async fn publish(cx: &Cx, author: String, code: String) -> Result<bool> {
    let author = author.trim();
    let author = if author.is_empty() { "Anonymous" } else { author };
    if author.chars().count() > 40 {
        return Ok(false);
    }
    let verdict = compiler::check(&code).await;
    if !verdict.compiles() {
        return Ok(false);
    }
    Post::add(cx, author, &code, verdict.millis).await?;
    Ok(true)
}
