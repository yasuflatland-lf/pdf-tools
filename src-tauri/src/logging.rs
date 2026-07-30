use std::error::Error;
use std::io;
use std::sync::OnceLock;

use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::EnvFilter;

const LOG_FILE_PREFIX: &str = "pdf-tools";
const LOG_FILE_SUFFIX: &str = "log";
const RETAINED_LOG_FILES: usize = 5;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn filter_directive(rust_log: Option<String>) -> String {
    rust_log.unwrap_or_else(|| "info".to_owned())
}

fn filter_directive_from_env() -> String {
    filter_directive(std::env::var("RUST_LOG").ok())
}

pub fn init(app: &tauri::App) -> Result<(), Box<dyn Error + Send + Sync>> {
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .max_log_files(RETAINED_LOG_FILES)
        .build(log_dir)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter_directive_from_env()))
        .with_writer(non_blocking)
        .try_init()?;

    LOG_GUARD.set(guard).map_err(|_| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "logging has already been initialized",
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{filter_directive, RETAINED_LOG_FILES};

    #[test]
    fn defaults_filter_directive_to_info() {
        assert_eq!(filter_directive(None), "info");
    }

    #[test]
    fn uses_rust_log_value_verbatim() {
        assert_eq!(
            filter_directive(Some("pdf_tools=debug,tauri=warn".to_owned())),
            "pdf_tools=debug,tauri=warn"
        );
    }

    #[test]
    fn retains_five_log_generations() {
        assert_eq!(RETAINED_LOG_FILES, 5);
    }
}
