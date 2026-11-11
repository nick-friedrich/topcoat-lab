//! Mission control for the Mac this server runs on.

use topcoat::{
    Result,
    context::{Cx, context},
    router::page,
    runtime::{Event, Signal, connected, procedure, shard, signal},
    view::{View, class, component, emit, live, view},
};

use crate::features::telemetry::{HISTORY, Snapshot, Telemetry};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    // Browser-only state: switching colors never talks to the server.
    let amber = signal(cx, || false);
    // Sent to the process table, which sorts on the server.
    let sort = signal(cx, || "cpu".to_owned());
    let message = signal(cx, String::new);

    Ok(view! {
        <main :class=$(if amber.get() { "mission amber" } else { "mission green" })>
            <div class="mx-auto max-w-7xl px-6 py-8">
                <header
                    class="mb-6 flex items-end justify-between border-b border-(--line) pb-4"
                >
                    <div>
                        <p class="text-xs tracking-[0.3em] text-(--dim) uppercase">
                            "Topcoat · live telemetry"
                        </p>
                        <h1 class="text-4xl font-bold tracking-tight text-(--phos)">
                            "MISSION CONTROL"
                        </h1>
                    </div>
                    <div class="flex items-center gap-4">
                        <span
                            class="flex items-center gap-2 text-xs tracking-widest text-(--dim)"
                        >
                            <span
                                class="pulse inline-block size-2 rounded-full bg-(--phos)"
                            ></span>
                            "LIVE"
                        </span>
                        <button
                            class="rounded border border-(--line) px-3 py-1 text-xs tracking-widest text-(--phos) hover:bg-(--glow)"
                            @click=$(|_e: Event| amber.toggle())
                        >
                            $(if amber.get() {
                                "PHOSPHOR: AMBER"
                            } else {
                                "PHOSPHOR: GREEN"
                            })
                        </button>
                    </div>
                </header>

                vitals()

                <section class="panel mt-6">
                    <div class="mb-3 flex items-center justify-between">
                        <h2 class="label">"Processes"</h2>
                        <div class="flex gap-2 text-xs">
                            <button
                                :class=$(if sort.get() == "cpu" { "tab on" } else { "tab" })
                                @click=$(|_e: Event| sort.set("cpu".to_owned()))
                            >
                                "BY CPU"
                            </button>
                            <button
                                :class=$(if sort.get() == "memory" {
                                    "tab on"
                                } else {
                                    "tab"
                                })
                                @click=$(|_e: Event| sort.set("memory".to_owned()))
                            >
                                "BY MEMORY"
                            </button>
                        </div>
                    </div>
                    <p class="mb-2 h-5 text-xs text-(--phos)">$(message.get())</p>
                    processes(sort: $(sort.get()), message: message)
                </section>
            </div>
        </main>
    })
}

/// Cores, CPU history, memory, battery. Streams a fresh render every second.
#[shard]
async fn vitals(cx: &Cx) -> Result<impl View> {
    Ok(live! {
        let telemetry = context::<Telemetry>(cx);
        let mut ticks = telemetry.subscribe();
        loop {
            let s = telemetry.latest();
            let token = emit! { vitals_panels(s: &s) }?;
            // The first render goes out over HTTP. Then the browser opens a
            // WebSocket, and this loop keeps sending new frames.
            if !connected(cx) {
                break Ok(token);
            }
            ticks.recv().await.ok();
        }
    })
}

#[component]
async fn vitals_panels(cx: &Cx, s: &Snapshot) -> Result<impl View> {
    let gb = |bytes: u64| format!("{:.1}", bytes as f64 / 1_073_741_824.0);
    Ok(view! {
        <div class="grid gap-6 lg:grid-cols-3">
            <section class="panel lg:col-span-2">
                <div class="flex items-baseline justify-between">
                    <h2 class="label">
                        "CPU · "
                        (&s.chip)
                        " · "
                        (s.cores.len())
                        " cores"
                    </h2>
                    <p class="big">
                        (format!("{:.0}", s.cpu))
                        <span class="unit">"%"</span>
                    </p>
                </div>
                sparkline(values: s.cpu_history.iter().copied().collect::<Vec<_>>())
                <div class="mt-4 flex h-28 gap-2">
                    for (i, load) in s.cores.iter().enumerate() {
                        <div
                            class="flex flex-1 flex-col items-stretch gap-1 text-center"
                        >
                            <div class="core relative w-full flex-1">
                                <div
                                    class="core-fill absolute bottom-0 w-full"
                                    style=(format!("height:{load:.0}%"))
                                ></div>
                            </div>
                            <span class="text-[10px] text-(--dim)">(i)</span>
                        </div>
                    }
                </div>
            </section>

            <section class="panel space-y-5">
                <div>
                    <h2 class="label">"Memory"</h2>
                    <p class="big">
                        (gb(s.memory_used))
                        <span class="unit">
                            " / "
                            (gb(s.memory_total))
                            " GB"
                        </span>
                    </p>
                    sparkline(
                        values: s.memory_history.iter().copied().collect::<Vec<_>>()
                    )
                </div>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <h2 class="label">"Load"</h2>
                        <p class="mid">(format!("{:.2}", s.load[0]))</p>
                        <p class="text-xs text-(--dim)">
                            (format!("{:.2} · {:.2}", s.load[1], s.load[2]))
                        </p>
                    </div>
                    <div>
                        <h2 class="label">"Battery"</h2>
                        if let Some(battery) = &s.battery {
                            <p class="mid">
                                (battery.percent)
                                "%"
                            </p>
                            <p class="text-xs text-(--dim) uppercase">
                                (&battery.state)
                            </p>
                        } else {
                            <p class="mid">"AC"</p>
                        }
                    </div>
                    <div class="col-span-2">
                        <h2 class="label">
                            "Uptime · "
                            (&s.host)
                        </h2>
                        <p class="mid">(uptime(s.uptime))</p>
                        <p class="text-xs text-(--dim)">(&s.os)</p>
                    </div>
                </div>
            </section>
        </div>
    })
}

/// A line chart of the last minute, drawn on the server as SVG.
#[component]
async fn sparkline(cx: &Cx, values: Vec<f32>) -> Result<impl View> {
    let step = 600.0 / (HISTORY - 1) as f32;
    let offset = (HISTORY - values.len()) as f32 * step;
    let points: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{:.1},{:.1}",
                offset + i as f32 * step,
                100.0 - v.clamp(0.0, 100.0)
            )
        })
        .collect();
    let line = points.join(" ");
    let area = match (points.first(), values.len()) {
        (Some(_), n) if n > 1 => format!("{offset:.1},100 {line} 600,100"),
        _ => String::new(),
    };
    Ok(view! {
        <svg
            class="spark mt-3 h-20 w-full"
            viewBox="0 0 600 100"
            preserveAspectRatio="none"
        >
            <polygon class="spark-area" points=(area)></polygon>
            <polyline class="spark-line" points=(line)></polyline>
        </svg>
    })
}

/// The busiest processes. Sorting happens here, on the server.
#[shard]
async fn processes(cx: &Cx, sort: String, message: Signal<String>) -> Result<impl View> {
    let by_memory = sort == "memory";
    Ok(live! {
        let telemetry = context::<Telemetry>(cx);
        let mut ticks = telemetry.subscribe();
        loop {
            let mut list = telemetry.latest().processes;
            if by_memory {
                list.sort_by(|a, b| b.memory.cmp(&a.memory));
            }
            list.truncate(10);
            let token = emit! {
                <table class="w-full text-sm">
                    <thead>
                        <tr class="text-left text-xs text-(--dim)">
                            <th class="py-1 font-normal">"PID"</th>
                            <th class="font-normal">"NAME"</th>
                            <th class="text-right font-normal">"CPU %"</th>
                            <th class="text-right font-normal">"MEMORY"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        #[key(p.pid)]
                        for p in list {
                            let pid = p.pid;
                            <tr
                                id=(format!("pid-{pid}"))
                                class="border-t border-(--line)"
                            >
                                <td class="py-1.5 text-(--dim)">(pid)</td>
                                <td class="max-w-64 truncate">(p.name)</td>
                                <td class=(class!("text-right", "hot" if p.cpu > 50.0))>
                                    (format!("{:.1}", p.cpu))
                                </td>
                                <td class="text-right">
                                    (format!("{:.0} MB", p.memory as f64 / 1_048_576.0))
                                </td>
                                <td class="text-right">
                                    if p.ours {
                                        <button
                                            class="stop"
                                            @click=$(async |_e: Event| {
                                                let reply = stop(pid).await;
                                                message.set(reply);
                                            })
                                        >
                                            "STOP"
                                        </button>
                                    }
                                </td>
                            </tr>
                        }
                    </tbody>
                </table>
            }?;
            if !connected(cx) {
                break Ok(token);
            }
            ticks.recv().await.ok();
        }
    })
}

/// Stops a process. Only ones started by this server's user.
#[procedure]
pub async fn stop(cx: &Cx, pid: u32) -> Result<String> {
    Ok(match context::<Telemetry>(cx).stop(pid) {
        Ok(done) => done,
        Err(refused) => refused,
    })
}

fn uptime(seconds: u64) -> String {
    let (d, h, m, s) = (
        seconds / 86_400,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60,
    );
    format!("{d}d {h:02}:{m:02}:{s:02}")
}
