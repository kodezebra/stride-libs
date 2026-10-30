//! Log setup: stderr for a terminal, one append-only file for launched apps.
//!
//! Rotation is deliberately not implemented. `tracing-appender` is not in the
//! local dependency cache and a desktop app that writes a few hundred lines per
//! session does not need daily files yet; add it when log size becomes a real
//! problem, not pre-emptively.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// Appends to a single log file, shared between all threads.
#[derive(Clone)]
struct FileWriter {
    file: Arc<Mutex<File>>,
}

impl FileWriter {
    fn open(path: &Path) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }
}

/// Holds the lock for the duration of one log event.
struct LockedFile<'a>(std::sync::MutexGuard<'a, File>);

impl Write for LockedFile<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<'a> MakeWriter<'a> for FileWriter {
    type Writer = LockedFile<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        // A poisoned lock means another thread panicked while logging. Losing
        // log lines is bad; losing the panic message because logging panicked
        // again is worse, so take the inner value.
        LockedFile(self.file.lock().unwrap_or_else(|error| error.into_inner()))
    }
}

/// Install the global subscriber. `RUST_LOG` overrides the default `info`.
pub fn init(app: &str, log_dir: &Path) -> anyhow::Result<()> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);

    let file = FileWriter::open(&log_dir.join(format!("{app}.log")))?;
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(file);

    tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init()?;

    Ok(())
}
