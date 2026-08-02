# CLAUDE.md — pdf-tools

Mac/Windows native desktop app (Tauri 2) that merges PDFs and images (jpg/png/gif) into a single PDF, entirely offline — no file ever leaves the machine and the app makes no network requests.
State: v0.5.0. The merge plan (an ordered list of one-page slots) is canonical in Rust, bound to its sources in a `MergeDocument` aggregate; every Tauri command returns a whole `PlanSnapshot` and Zustand only replaces its contents. Sources arrive by drop, by a file or folder picker in the empty state, or from the toolbar `Add` menu; a folder is resolved by `expand_paths` into every supported file beneath it in natural order, which changes no plan, so a large count can be confirmed before anything enters the document. A source that contributes no pages — an encrypted or damaged file — is dismissed with `remove_source` rather than left on screen. Each slot carries a clockwise quarter-turn `Rotation` that `compose` writes as the PDF page's `/Rotate` attribute, so a turn re-encodes nothing, never costs the JPEG passthrough, and changes no page but the one it was applied to: every sheet is sized at its representative unrotated size. Contiguous pages from one source collapse into a card, ungroup when a drag leaves the run non-contiguous or no longer ascending or a rotation leaves it mixed, and refold automatically once the run is contiguous, monotonically increasing and uniformly rotated again. Thumbnails cross IPC as raw PNG bytes into a virtualized grid backed by an LRU blob cache. `compose` passes eligible JPEG streams through untouched (100 images in 0.21 s) and decodes every other image, re-embedding it under a 200 DPI cap; both paths merge 100 pages inside the 10 s target at the scale that target names. Known limitation: far past that scale — 900 MB of PNG — a merge still takes ~11.6 s, because what remains is PNG decode rather than embedding.

## Mandatory rules (harness)

- **Write everything in this repository in English** — code comments, identifiers, on-screen strings, commit messages, and **the title and body of every pull request and issue**.
- **TDD**: write the failing test first, then the smallest implementation (cargo-nextest / Vitest).
- **Layer boundaries**: `domain` is pure (no IO/async/external crates); IO lives behind the `PdfEngine`/`ImageDecoder` ports; deps flow presentation→application→domain and infrastructure→domain, never back.
- **Do not swap libraries** (Tauri 2 / pdfium-render / image / @tanstack/react-virtual / @dnd-kit / oxlint / oxfmt are fixed choices).
- **One PR = one issue, ≤ 800 lines of production code** (tests excluded); clippy/fmt/nextest/tsc/oxlint/oxfmt/knip/vitest all green before merge.
- **Commit messages are a single line**, no body, no trailers, and no AI attribution of any kind.
- **Never commit the PDFium binary** (`src-tauri/resources/pdfium/`); fetch it with `mise run fetch-pdfium`.
- Regenerate and commit `src/bindings/**` (ts-rs) in the same PR as any DTO change.

## Documentation layers (L1/L2/L3)

- **L1 = this file**: mandatory rules + doc map only (keep under 35 lines).
- **L2 = `docs/`**: how the project works.
  - `docs/architecture.md` — layering, boundaries, ports, state ownership, measured vs estimated SLOs
  - `docs/development.md` — tech stack, dev commands, testing policy, PR rules
- **L3 = `.claude/`**: deep per-task reference — add files here rather than growing L1.
  - `.claude/macos-code-signing.md` — the self-signed signing certificate: generating it, registering the secrets, verifying it before a release run
  - `.claude/numeric-precision.md` — why page geometry is `f32`, why arbitrary precision is barred from `domain`, and what to change first if precision ever bites

## Source of truth

The authoritative design lives in Notion; the local mirror is `docs/superpowers/specs/`, which is git-ignored and never ships. Documentation meant to ship belongs in `docs/architecture.md` or `docs/development.md`.
If these docs conflict with the design, treat the design as canonical and propose an update.
