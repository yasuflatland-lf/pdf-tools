# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet.

## [0.2.0] — 2026-08-01

Both image merge paths now meet the 10 s objective, the interface is rebuilt around its toolbar
icons and reads English throughout, and the Rust domain is restructured around a single
`MergeDocument` aggregate.

Every measured figure below comes from issue #68
([the objective-scale run](https://github.com/yasuflatland-lf/pdf-tools/issues/68#issuecomment-5137691880)
and [the worst-case run](https://github.com/yasuflatland-lf/pdf-tools/issues/68#issuecomment-5137808149));
nothing here is estimated.

### Merging

- Eligible one- and three-component JPEGs are embedded as their original `DCTDecode` stream.
  Other images are decoded and capped at 200 DPI for their occupied page area; four-component
  JPEGs are deliberately excluded from passthrough because PDFium renders their colours
  incorrectly.
- On an Apple M5 Pro running macOS 26.5.2, a `--release` build with PDFium `chromium/7961` merged
  100 synthetic 2048×1536 JPEGs by passthrough in 0.21 s (203.7 MB input and output). At the
  scale the 10 s objective names, the capped raster path merged 100 photo-like pages of the same
  dimensions in 7.48 s (43.8 MB in, 111.7 MB out). **Both paths now meet the objective.**
- The 200 DPI cap bounds the worst case rather than the common one. On a deliberately noisy
  900.2 MB PNG corpus — nine times the objective's size — it cut the measured result from 13.23 s
  and 858.7 MB to 11.63 s and 543.0 MB. That corpus stays above 10 s: what is left there is PNG
  decode rather than embedding, and no cap can reduce it.
- **The dominant page size no longer depends on the order files were dropped in.** Sizes used to
  be grouped by a tolerance comparison, which is not transitive: 595.0 pt matched 595.5 pt and
  595.5 pt matched 596.0 pt, yet 595.0 pt did not match 596.0 pt, so which size won depended on
  which page happened to be examined first. Each dimension is now rounded onto a 1 pt lattice,
  which is an equivalence relation, and all six orderings of that witness agree.

### Interface

- The toolbar is rebuilt around its icons. The tools sit in a group held against the window
  midpoint, so they stay put while the regions beside them change width; the file and page counts
  become a quiet readout on the left, and `Merge` keeps the right edge to itself. Merge progress
  moves onto the bar's bottom edge, where it costs no horizontal space. The redundant application
  header above it is gone.
- The window will not shrink below 960 px, which is what stops a long output file name from
  sliding underneath the centred tools.
- **Card focus follows the card, not the position.** The focus ring used to stay at a fixed index
  while the plan changed underneath it, so after a delete the ring sat on a card that was not
  selected and the next `Delete` acted on a selection the user could no longer see. Focus is now
  tracked by card identity, and when the focused card is removed the ring and the selection move
  together onto the card that replaced it.
- **A command that changes nothing no longer arms Undo.** Dropping zero files, or a reorder that
  puts a card back where it started, used to push an undo entry and discard the redo stack.
- **Every on-screen string is English.** Eight error and status strings were still Japanese while
  every control and every `aria-label` around them was English — in two components the same
  element carried both languages at once.
- An unreadable file's card names the reason instead of interpolating the PDF engine's own
  message and a full filesystem path into the badge. A corrupt file now reads
  `This file is damaged and could not be read`, and the engine's own message is kept in the log
  at warn level.
- On Windows the page grid counts its columns from the element that actually scrolls. Measuring
  the wrapper around it counted the scrollbar's width as usable space, which could produce one
  column too many at particular window widths and cards narrower than their 180 px minimum.
  macOS overlay scrollbars never showed this.

### Reliability

- A poisoned plan-session lock is recovered from rather than propagated, and that choice is now
  deliberate: the recovery is documented on the accessor, logged at warn level, and pinned by a
  regression test. Propagating it would have cost the user the whole document to a forced
  restart.

### Keyboard and selection

- The merge-failure dialog owns the keyboard while it is open, and the list view now matches the
  grid's arrow-key focus and selection behaviour.
- The window-level shortcuts are served by one listener instead of one per component, so the
  handler set no longer detaches and re-attaches as cards render.

### Thumbnails

- The grid and list share the same thumbnail effect, and a failed thumbnail is visibly marked.

### Architecture

- `MergePlan` and its sources are bound into one `MergeDocument` aggregate. Every slot is
  guaranteed by construction to name a listed source, which removed the four call sites that each
  defended against that case independently and gave `source_of` a total signature.
- Whether a source is grouped is derived from the plan rather than stored on `SourceFile`. The
  field was never state: its only writer recomputed it from the plan after every mutating command.
- `SourceFileDto`'s `kind` and `grouping` cross the IPC boundary as generated union types instead
  of bare strings, so a typo in a frontend comparison is a type error rather than a silent
  mismatch. `SourceStatus::Unreadable` likewise carries a typed reason instead of free text.
- The grid and the list share one `CardSurface`; they were the same component twice and had begun
  to drift. The unreachable `insert_at` and `pdfium_health` commands are removed, so every
  registered command has a caller.
- The undo history's bound and the fact that a source holding a duplicated page stays ungrouped
  are now written down in `docs/architecture.md`. Both were behaviours a user could reach and
  neither was documented.

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
  13.1–13.6 s and produce an 858.9 MB file, because every image is decoded and re-embedded as a
  raw bitmap, which PDFium filters with `FlateDecode` when it saves — deflating photographic
  pixels is slow and barely compresses them. Merging PDF pages alone is effectively free (100
  pages in 0.001 s). Passing the original JPEG stream through, or embedding fewer pixels, is the
  fix.
- **The progress bar does not advance page by page.** Every progress tick is emitted before the
  merge engine starts, so the bar reaches 100% at once and the window then waits without further
  feedback — most visible on the slow image path above.
- **Scroll frame rate, real peak memory and cold start are not measured.** They are properties of
  the packaged app in a WebView, and every measurement so far was taken headlessly. See
  [`docs/architecture.md` § Service level objectives](docs/architecture.md#service-level-objectives)
  for what was measured and what was not.
- No PDF editing, splitting, OCR, or password-protected PDF support — all out of scope by design.
- Linux is not supported.
