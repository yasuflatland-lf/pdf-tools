pub mod logging;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(error) = logging::init(app) {
                eprintln!("failed to initialize logging: {error}");
            } else {
                tracing::info!(
                    version = %env!("CARGO_PKG_VERSION"),
                    "pdf-tools starting"
                );
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
