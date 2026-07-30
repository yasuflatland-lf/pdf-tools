# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-31

First release. The version is set to `0.1.0` across `Cargo.toml` (both crates), `package.json` and
`tauri.conf.json`.

PDF Tools merges PDFs and images (jpg / png / gif) into a single PDF, in any page order, without a
single byte leaving the machine.

### Merging

- Merge any mix of PDFs and images into one PDF, driven by a **merge plan**: an ordered list of
  slots where one slot is exactly one output page.
- Image pages are fitted to the **dominant page size** of the plan's PDF pages — the most frequent
  size, ties broken by first appearance, A4 portrait when the plan holds no PDF pages. The aspect
  ratio is preserved and the remaining area is white. Animated GIFs contribute their first frame.
- The merge runs off the command thread, so the interface stays responsive while it works, and
  `compose-progress` events report the page total to the UI.
- The output directory is remembered between runs, defaulting to the OS Downloads folder.

### Building the plan

- Drag and drop files onto the window, or add them from the toolbar; each file is probed and its
  pages are appended to the plan.
- Contiguous pages from one file collapse into a **single card** showing how many pages it
  currently holds. Inserting another file's page inside that run expands it into per-page cards;
  deleting back to a contiguous, monotonically increasing run folds it up again automatically.
- Reorder by dragging, in either a **grid** or a **list** view; the choice persists.
- Select with the mouse or keyboard, and delete selected pages.
- **Undo and redo** cover every plan edit, from the toolbar or with `Cmd/Ctrl+Z` and
  `Cmd/Ctrl+Shift+Z`. Arrow keys move card focus, `Cmd/Ctrl+A` selects all, `Escape` clears the
  selection, `Delete`/`Backspace` removes the selection — and a keystroke typed into a text field
  always edits the text instead.

### Large inputs

- The page grid is **virtualized** and thumbnails are rasterized only for the visible range, so
  the number of render requests stays constant as the plan grows.
- Thumbnails are cached as blob URLs behind an **LRU cache** that revokes each URL on eviction,
  bounding memory rather than letting it grow with the plan.
- Thumbnails cross the IPC boundary as raw PNG bytes, never base64.

### Error handling

- An **encrypted or corrupt** file is flagged on its card with the reason and excluded from the
  merge; the rest of the batch still merges.
- A file that was moved or deleted after being added is caught at merge time, and the failure
  dialog names the offending file.
- If PDFium fails to load, the app still starts and reports the failure per operation instead of
  refusing to open.

### Architecture

- Two-crate Cargo workspace: a pure `pdf-tools-core` (domain / application / infrastructure) and
  the `src-tauri` presentation crate. Dependencies flow presentation → application → domain and
  infrastructure → domain only.
- PDFium sits behind a three-operation `PdfEngine` port (probe / rasterize / compose) that speaks
  in paths and page indices and never exposes a document handle; `FakePdfEngine` is the second,
  deterministic implementation that keeps the port honest.
- Plan state is canonical in Rust — including the undo/redo stacks — and every command returns a
  whole snapshot, so the frontend never reimplements a merge rule.
- Rust DTOs generate the TypeScript bindings via `ts-rs`; the generated files are committed and
  type-checked in CI, so a DTO change cannot drift silently.
- Domain invariants are covered by property tests: slots are preserved across operations, redo
  after undo is the identity, and automatic regrouping fires only for a contiguous, increasing run.

### Platform, packaging and privacy

- macOS (universal: Apple Silicon + Intel) and Windows bundles, built by tagged release workflow.
- The PDFium binary is fetched from a **pinned** release and verified by SHA-256 — a mismatch
  fails the build — and is never committed to the repository.
- On macOS the PDFium dylib is signed before the app is sealed, and the signature of both the
  `.app` and the `.app` inside the `.dmg` is verified in CI.
- PDFium's third-party license text ships with the bundle.
- **No network requests, no telemetry, no crash reporting.** Logs record file paths and outcomes,
  never file contents.

### Known limitations

- **Merging images is slower than the 10 s objective.** 100 JPEGs at 2048×1536 (93.8 MB) take
  13.1–13.6 s and produce an 858.9 MB file, because images are embedded as uncompressed bitmaps.
  Merging PDF pages alone is effectively free (100 pages in 0.001 s). Passing the original JPEG
  stream through, or compressing on embed, is the fix.
- **The progress bar does not advance page by page.** Every progress tick is emitted before the
  merge engine starts, so the bar reaches 100% at once and the window then waits without further
  feedback — most visible on the slow image path above.
- **Scroll frame rate, real peak memory and cold start are not measured.** They are properties of
  the packaged app in a WebView, and every measurement so far was taken headlessly. See
  [`docs/architecture.md` § Service level objectives](docs/architecture.md#service-level-objectives)
  for what was measured and what was not.
- No PDF editing, splitting, OCR, or password-protected PDF support — all out of scope by design.
- Linux is not supported.
