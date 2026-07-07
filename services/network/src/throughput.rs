use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::debug;

/// Tracks byte counters for throughput calculation.
pub struct ThroughputTracker {
    last_rx_bytes: u64,
    last_tx_bytes: u64,
    last_sample_time: Instant,
}

impl ThroughputTracker {
    pub fn new() -> Self {
        Self {
            last_rx_bytes: 0,
            last_tx_bytes: 0,
            last_sample_time: Instant::now(),
        }
    }

    /// Reads aggregate RX/TX bytes from all network interfaces via `/proc/net/dev`.
    pub fn read_aggregate_bytes() -> (u64, u64) {
        let content = match std::fs::read_to_string("/proc/net/dev") {
            Ok(c) => c,
            Err(e) => {
                debug!("Network Service: failed to read /proc/net/dev: {e}");
                return (0, 0);
            }
        };

        let mut total_rx: u64 = 0;
        let mut total_tx: u64 = 0;

        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }
            let rx_bytes: u64 = parts[1].parse().unwrap_or(0);
            let tx_bytes: u64 = parts[9].parse().unwrap_or(0);
            total_rx += rx_bytes;
            total_tx += tx_bytes;
        }

        (total_rx, total_tx)
    }

    /// Calculates throughput in bytes per second since the last sample.
    pub fn sample(&mut self) -> (u64, u64) {
        let (rx_bytes, tx_bytes) = Self::read_aggregate_bytes();
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time).as_secs_f64();

        if elapsed < 0.1 {
            return (0, 0);
        }

        let rx_bps = if rx_bytes >= self.last_rx_bytes {
            ((rx_bytes - self.last_rx_bytes) as f64 / elapsed) as u64
        } else {
            0
        };
        let tx_bps = if tx_bytes >= self.last_tx_bytes {
            ((tx_bytes - self.last_tx_bytes) as f64 / elapsed) as u64
        } else {
            0
        };

        self.last_rx_bytes = rx_bytes;
        self.last_tx_bytes = tx_bytes;
        self.last_sample_time = now;

        (rx_bps, tx_bps)
    }
}

impl Default for ThroughputTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared throughput state used by the service event loop.
pub type SharedThroughputTracker = Arc<Mutex<ThroughputTracker>>;
