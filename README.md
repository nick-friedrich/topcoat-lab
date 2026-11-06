# topcoat-lab

Two small apps built with [Topcoat](https://github.com/tokio-rs/topcoat) 0.11, Tokio's full-stack Rust framework, served by one binary. Made for a YouTube video.

- **It Compiles** (`/wall`): a guestbook where every message is a Rust program. A shard pipes your code into `rustc --emit=metadata --error-format=json` on every keystroke and renders the diagnostics; you can only post once it compiles. Code is type-checked, never run.
- **Mission Control** (`/mission`): live CPU, memory, load, battery and process stats of the machine running the server, streamed with `live!`/`emit!` over a WebSocket once a second. The STOP button sends SIGTERM to processes owned by the same user.

> Both apps run rustc on, and send signals from, the machine they are served on. This is a local demo, not something to put on the open internet.

Requires Rust 1.99 or newer (`rustup update stable`) and macOS for the battery reading (`pmset`).

## Development

Install the Topcoat CLI:

```sh
cargo install topcoat-cli
```

Create the database tables by generating a migration from the models and applying it:

```sh
cargo run -- toasty migration generate
cargo run -- toasty migration apply
```

Run both commands again after changing a model. The database is stored in `data.db`.

Start the development server, which rebuilds the application when you edit its source:

```sh
topcoat dev
```

Then open <http://127.0.0.1:3000>.

## Project structure

| Path | Contents |
| --- | --- |
| `src/app.rs`, `src/app/` | The route tree: the router, layouts, pages, and routes. |
| `src/features.rs`, `src/features/` | Everything one feature owns, such as its data and business rules. |
| `src/components.rs`, `src/components/` | Shared UI components that belong to no single feature. |
| `src/db.rs` | The database connection. |
| `toasty/` | The database migrations. Commit them with your code. |

Pages in `app` use features and components. A feature can use shared components but never imports from `app`, and shared components never import from a feature.

## Learn more

- [Getting started](https://github.com/tokio-rs/topcoat/blob/main/docs/getting_started.md)
- [Guides](https://github.com/tokio-rs/topcoat/tree/main#learn-topcoat)
