//! Samples this Mac once a second.
//!
//! One background task does the sampling. Pages only read the latest snapshot and wait
//! for the next one, so a hundred open tabs still cost one sampler.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tokio::sync::broadcast;

/// How many seconds of history the charts show.
pub const HISTORY: usize = 60;

/// Everything the dashboard shows, at one moment.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub host: String,
    pub chip: String,
    pub os: String,
    pub uptime: u64,
    pub cores: Vec<f32>,
    pub cpu: f32,
    pub cpu_history: VecDeque<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub memory_history: VecDeque<f32>,
    pub load: [f64; 3],
    pub battery: Option<Battery>,
    pub processes: Vec<Proc>,
}

#[derive(Clone)]
pub struct Battery {
    pub percent: u8,
    pub state: String,
}

#[derive(Clone)]
pub struct Proc {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory: u64,
    /// Started by the same user as this server, so the dashboard may stop it.
    pub ours: bool,
}

/// Shared by the sampler and every page. Cheap to clone.
#[derive(Clone)]
pub struct Telemetry {
    latest: Arc<Mutex<Snapshot>>,
    ticks: broadcast::Sender<()>,
}

impl Telemetry {
    /// Starts the sampler in the background.
    pub fn start() -> Self {
        let telemetry = Telemetry {
            latest: Arc::default(),
            ticks: broadcast::channel(4).0,
        };
        tokio::spawn(telemetry.clone().sample_forever());
        telemetry
    }

    pub fn latest(&self) -> Snapshot {
        self.latest.lock().unwrap().clone()
    }

    /// Fires after every new sample. Subscribe before reading `latest`.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.ticks.subscribe()
    }

    /// Sends SIGTERM to a process, if this server's user owns it.
    pub fn stop(&self, pid: u32) -> Result<String, String> {
        let mut system = System::new().map_err(|e| e.to_string())?;
        system.refresh_processes(ProcessesToUpdate::All, true);
        let me = system
            .process(sysinfo::get_current_pid().unwrap())
            .and_then(|p| p.user_id().cloned());
        let process = system
            .process(Pid::from_u32(pid))
            .ok_or("That process already exited.")?;
        if process.user_id().cloned() != me || pid == std::process::id() {
            return Err("Not one of ours.".into());
        }
        let name = process.name().to_string_lossy().into_owned();
        match process.kill_with(Signal::Term) {
            Some(true) => Ok(format!("Sent SIGTERM to {name} ({pid}).")),
            _ => Err(format!("Couldn't stop {name}.")),
        }
    }

    async fn sample_forever(self) {
        let mut system = System::new_all().expect("sysinfo supports macOS");
        let me = system
            .process(sysinfo::get_current_pid().unwrap())
            .and_then(|p| p.user_id().cloned());
        let mut battery = read_battery().await;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        for tick in 0u64.. {
            interval.tick().await;
            system.refresh_cpu_usage();
            system.refresh_memory();
            system.refresh_processes(ProcessesToUpdate::All, true);
            if tick % 10 == 0 {
                battery = read_battery().await;
            }

            let mut processes: Vec<Proc> = system
                .processes()
                .values()
                .map(|p| Proc {
                    pid: p.pid().as_u32(),
                    name: p.name().to_string_lossy().into_owned(),
                    cpu: p.cpu_usage(),
                    memory: p.memory(),
                    ours: p.user_id().cloned() == me && p.pid().as_u32() != std::process::id(),
                })
                .collect();
            processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            processes.truncate(40);

            let mut snapshot = self.latest.lock().unwrap();
            let memory = system.used_memory() as f32 / system.total_memory() as f32 * 100.0;
            push(&mut snapshot.cpu_history, system.global_cpu_usage());
            push(&mut snapshot.memory_history, memory);
            let load = System::load_average()
                .map(|l| [l.one, l.five, l.fifteen])
                .unwrap_or_default();
            *snapshot = Snapshot {
                host: System::host_name()
                    .unwrap_or_default()
                    .trim_end_matches(".fritz.box")
                    .to_owned(),
                chip: system
                    .cpus()
                    .first()
                    .map(|c| c.brand().to_owned())
                    .unwrap_or_default(),
                os: System::long_os_version().unwrap_or_default(),
                uptime: System::uptime().unwrap_or(0),
                cores: system.cpus().iter().map(|c| c.usage()).collect(),
                cpu: system.global_cpu_usage(),
                cpu_history: std::mem::take(&mut snapshot.cpu_history),
                memory_used: system.used_memory(),
                memory_total: system.total_memory(),
                memory_history: std::mem::take(&mut snapshot.memory_history),
                load,
                battery: battery.clone(),
                processes,
            };
            drop(snapshot);
            let _ = self.ticks.send(());
        }
    }
}

fn push(history: &mut VecDeque<f32>, value: f32) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

/// macOS has no battery API in sysinfo, so ask `pmset`.
async fn read_battery() -> Option<Battery> {
    let output = tokio::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|l| l.contains("InternalBattery"))?;
    let fields = line.split('\t').nth(1)?;
    let mut parts = fields.split(';').map(str::trim);
    let percent = parts.next()?.trim_end_matches('%').parse().ok()?;
    let state = parts.next()?.to_owned();
    Some(Battery { percent, state })
}
