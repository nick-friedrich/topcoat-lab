//! Checks a snippet of Rust with the real compiler.
//!
//! The wall never runs anyone's code. It only asks `rustc` whether the code compiles,
//! and stops it after a few seconds.

use std::{
    process::Stdio,
    sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use serde::Deserialize;
use tokio::{io::AsyncWriteExt, process::Command, sync::Semaphore};

/// Longest snippet the wall accepts, in bytes.
pub const MAX_LEN: usize = 4_000;

/// At most four compilers run at the same time.
static SLOTS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(4));

/// What rustc thought of a snippet.
pub struct Verdict {
    pub diagnostics: Vec<Diagnostic>,
    pub millis: u64,
}

impl Verdict {
    pub fn compiles(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.level == "error")
    }

    fn refused(reason: &str) -> Self {
        Verdict {
            diagnostics: vec![Diagnostic {
                level: "error".into(),
                code: None,
                message: reason.into(),
                explanation: None,
                line: None,
                notes: vec![],
            }],
            millis: 0,
        }
    }
}

/// One error or warning, with the line it points at.
pub struct Diagnostic {
    pub level: String,
    pub code: Option<String>,
    pub message: String,
    /// The first sentence of `rustc --explain`.
    pub explanation: Option<String>,
    pub line: Option<SourceLine>,
    pub notes: Vec<String>,
}

/// The source line under a diagnostic, and where to draw the `^^^`.
pub struct SourceLine {
    pub number: usize,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub label: Option<String>,
}

/// Runs `rustc` on the snippet and collects what it says.
pub async fn check(code: &str) -> Verdict {
    if code.len() > MAX_LEN {
        return Verdict::refused("That's a long message. Keep it under 4,000 bytes.");
    }
    // These would let a snippet read files or environment variables on the server.
    for banned in ["include", "env!", "#[path", "#![", "mod "] {
        if code.contains(banned) {
            return Verdict::refused(&format!("The wall doesn't allow `{banned}`."));
        }
    }

    let _slot = SLOTS.acquire().await.unwrap();
    let started = Instant::now();

    // rustc writes a small metadata file and needs a writable folder for it.
    static RUNS: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join("topcoat-wall");
    let _ = std::fs::create_dir_all(&dir);
    let out = dir.join(format!("{}.rmeta", RUNS.fetch_add(1, Ordering::Relaxed)));

    let child = Command::new("rustc")
        .args(["--edition=2024", "--crate-name=wall"])
        .arg(format!("--emit=metadata={}", out.display()))
        .args(["--error-format=json", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let Ok(mut child) = child else {
        return Verdict::refused("rustc isn't installed on this server.");
    };

    let mut stdin = child.stdin.take().unwrap();
    let _ = stdin.write_all(code.as_bytes()).await;
    drop(stdin);

    let output = match tokio::time::timeout(Duration::from_secs(5), child.wait_with_output()).await
    {
        Ok(Ok(output)) => output,
        _ => return Verdict::refused("rustc took longer than five seconds."),
    };
    let _ = std::fs::remove_file(&out);

    let diagnostics = String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| serde_json::from_str::<Json>(line).ok())
        // Skip the closing "aborting due to 1 previous error" summary.
        .filter(|d| {
            !d.spans.is_empty() || (d.level == "error" && !d.message.starts_with("aborting"))
        })
        .map(Diagnostic::from)
        .collect();

    Verdict {
        diagnostics,
        millis: started.elapsed().as_millis() as u64,
    }
}

// rustc's JSON diagnostics, only the fields the wall shows.

#[derive(Deserialize)]
struct Json {
    message: String,
    level: String,
    code: Option<JsonCode>,
    spans: Vec<JsonSpan>,
    children: Vec<JsonChild>,
}

#[derive(Deserialize)]
struct JsonCode {
    code: String,
    explanation: Option<String>,
}

#[derive(Deserialize)]
struct JsonSpan {
    line_start: usize,
    is_primary: bool,
    label: Option<String>,
    text: Vec<JsonText>,
}

#[derive(Deserialize)]
struct JsonText {
    text: String,
    highlight_start: usize,
    highlight_end: usize,
}

#[derive(Deserialize)]
struct JsonChild {
    message: String,
    level: String,
}

impl From<Json> for Diagnostic {
    fn from(json: Json) -> Self {
        let span = json
            .spans
            .iter()
            .find(|s| s.is_primary)
            .or(json.spans.first());
        let line = span.and_then(|span| {
            let text = span.text.first()?;
            Some(SourceLine {
                number: span.line_start,
                text: text.text.clone(),
                start: text.highlight_start.saturating_sub(1),
                end: text.highlight_end.saturating_sub(1),
                label: span.label.clone(),
            })
        });
        let (code, explanation) = match json.code {
            Some(code) => {
                let first = code
                    .explanation
                    .and_then(|e| e.lines().next().map(str::to_owned));
                (Some(code.code), first.filter(|line| !line.is_empty()))
            }
            None => (None, None),
        };
        Diagnostic {
            level: json.level,
            code,
            message: json.message,
            explanation,
            line,
            notes: json
                .children
                .into_iter()
                // Keep hints, skip "`#[warn(...)]` on by default".
                .filter(|c| c.level == "help" || (c.level == "note" && !c.message.starts_with('`')))
                .map(|c| format!("{}: {}", c.level, c.message))
                .collect(),
        }
    }
}
