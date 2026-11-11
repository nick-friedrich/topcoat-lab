//! The shared pixel board.
//!
//! One board lives in memory for the whole server. Every open page subscribes to it,
//! and every paint, from a browser or from the API, notifies all of them.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use tokio::sync::broadcast;

pub const WIDTH: usize = 40;
pub const HEIGHT: usize = 24;

/// The colors a pixel can have. Index 0 is an empty pixel.
pub const PALETTE: [&str; 12] = [
    "#f4efe6", "#1d1b22", "#e5484d", "#f76b15", "#ffc53d", "#46a758", "#12a594", "#0090ff",
    "#6e56cf", "#d6409f", "#8d6e63", "#ffffff",
];

/// Shared by every page and API route. Cheap to clone.
#[derive(Clone)]
pub struct Board {
    inner: Arc<Inner>,
}

struct Inner {
    pixels: Mutex<Vec<u8>>,
    paints: AtomicUsize,
    online: AtomicUsize,
    changed: broadcast::Sender<()>,
}

impl Default for Board {
    fn default() -> Self {
        Board {
            inner: Arc::new(Inner {
                pixels: Mutex::new(vec![0; WIDTH * HEIGHT]),
                paints: AtomicUsize::new(0),
                online: AtomicUsize::new(0),
                changed: broadcast::channel(64).0,
            }),
        }
    }
}

impl Board {
    pub fn pixels(&self) -> Vec<u8> {
        self.inner.pixels.lock().unwrap().clone()
    }

    pub fn paints(&self) -> usize {
        self.inner.paints.load(Ordering::Relaxed)
    }

    pub fn online(&self) -> usize {
        self.inner.online.load(Ordering::Relaxed)
    }

    /// Fires after every change. Subscribe before reading the board.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.inner.changed.subscribe()
    }

    /// Paints one pixel. Returns false when the pixel or color is out of range.
    pub fn paint(&self, x: usize, y: usize, color: u8) -> bool {
        if x >= WIDTH || y >= HEIGHT || color as usize >= PALETTE.len() {
            return false;
        }
        self.inner.pixels.lock().unwrap()[y * WIDTH + x] = color;
        self.inner.paints.fetch_add(1, Ordering::Relaxed);
        self.notify();
        true
    }

    pub fn clear(&self) {
        self.inner.pixels.lock().unwrap().fill(0);
        self.notify();
    }

    /// Counts a connected page until the returned guard is dropped.
    pub fn join(&self) -> Presence {
        self.inner.online.fetch_add(1, Ordering::Relaxed);
        self.notify();
        Presence(self.clone())
    }

    fn notify(&self) {
        // Nobody listening is fine.
        let _ = self.inner.changed.send(());
    }
}

/// One connected page. Closing the tab drops it, which updates the online count.
pub struct Presence(Board);

impl Drop for Presence {
    fn drop(&mut self) {
        self.0.inner.online.fetch_sub(1, Ordering::Relaxed);
        self.0.notify();
    }
}
