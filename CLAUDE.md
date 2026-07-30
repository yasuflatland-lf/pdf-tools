# CLAUDE.md — pdf-tools

Mac/Windows native desktop app (Tauri 2) that merges drag-and-dropped PDFs and images (jpg/png/gif) into a single PDF, entirely offline — no file ever leaves the machine and the app makes no network requests.
State: v0.1.0. The merge plan (an ordered list of one-page slots) is canonical in Rust; every Tauri command returns a whole `PlanSnapshot` and Zustand only replaces its contents. Contiguous pages from one source collapse into a card, ungroup when something is inserted inside the run, and refold automatically once the run is contiguous and monotonically increasing again. Thumbnails cross IPC as raw PNG bytes into a virtualized grid backed by an LRU blob cache. Known limitation: `compose` embeds images as uncompressed bitmaps, so a 100-image merge takes ~13 s against a 10 s target.

## Mandatory rules (harness)

- **Write all code comments in English.** Name identifiers in English too.
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
- **L3 = `.claude/`**: deep per-task reference. Not populated yet — add files here rather than growing L1.

## Source of truth

The authoritative design lives in Notion; the local mirror is `docs/superpowers/specs/`, which is git-ignored and never ships. Documentation meant to ship belongs in `docs/architecture.md` or `docs/development.md`.
If these docs conflict with the design, treat the design as canonical and propose an update.
