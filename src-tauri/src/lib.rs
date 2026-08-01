pub mod logging;
pub mod presentation;

use pdf_tools_core::application::ports::PdfEngine;
use pdf_tools_core::infrastructure::fs_walker::StdFsWalker;
use pdf_tools_core::infrastructure::image_decoder::ImageCrateDecoder;
use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use presentation::commands::{
    add_sources, compose, expand_paths, rasterize_slot, redo, remove_slots, remove_source, reorder,
    rotate_slots, supported_extensions, undo,
};
use presentation::state::{AppState, UnavailablePdfEngine};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

fn load_pdfium(app: &tauri::App) -> Result<Arc<PdfiumEngine>, String> {
    let mut candidates = Vec::new();
    let mut last_error = None;

    match app.path().resource_dir() {
        Ok(resource_dir) => candidates.push(resource_dir.join("resources/pdfium")),
        Err(error) => {
            last_error = Some(format!(
                "failed to resolve the PDFium resource directory: {error}"
            ));
        }
    }

    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("pdfium"),
    );

    for library_dir in candidates {
        match PdfiumEngine::new(&library_dir) {
            Ok(engine) => {
                tracing::info!(
                    path = %library_dir.display(),
                    version = %engine.version(),
                    "PDFium loaded"
                );
                return Ok(Arc::new(engine));
            }
            Err(error) => {
                tracing::warn!(
                    path = %library_dir.display(),
                    %error,
                    "failed to load PDFium"
                );
                last_error = Some(error.to_string());
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "PDFium engine is unavailable".to_owned()))
}

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

            let pdfium = load_pdfium(app);
            if let Err(error) = &pdfium {
                tracing::error!(%error, "PDFium is unavailable");
            }
            // The app still has to start when PDFium is missing, so the state
            // falls back to an engine that reports the load failure per call.
            let pdf_engine: Arc<dyn PdfEngine> = match &pdfium {
                Ok(engine) => engine.clone(),
                Err(error) => Arc::new(UnavailablePdfEngine::new(error.clone())),
            };
            app.manage(AppState::with_ports(
                pdf_engine,
                Arc::new(ImageCrateDecoder),
                Arc::new(StdFsWalker),
            ));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_sources,
            expand_paths,
            supported_extensions,
            reorder,
            remove_slots,
            remove_source,
            rotate_slots,
            undo,
            redo,
            rasterize_slot,
            compose
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
