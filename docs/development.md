# Development Guide (L2)

> Doc layers: L1 = [`/CLAUDE.md`](../CLAUDE.md) / L2 = this file + [`architecture.md`](architecture.md).
> The authoritative design lives in Notion; the local mirror is `docs/superpowers/specs/` (git-ignored).

## Tech stack

| Area                        | Choice                                                                                    |
| --------------------------- | ----------------------------------------------------------------------------------------- |
| Framework                   | Tauri 2 — one native app for macOS (universal) and Windows                                |
| Frontend                    | Vite 7 + React 19 + TypeScript 5.9 + Tailwind v4 (`@tailwindcss/vite`) + Zustand 5        |
| Virtualization / DnD        | `@tanstack/react-virtual` 3, `@dnd-kit/core` + `@dnd-kit/sortable`                        |
| Backend                     | Rust, DDD layered — workspace of pure `pdf-tools-core` + `src-tauri` presentation crate   |
| PDF engine                  | PDFium via `pdfium-render` 0.9; the binary is a pinned prebuilt from `pdfium-binaries`    |
| Image decoding              | `image` 0.25 (jpeg / png / gif features only)                                             |
| Rust tests                  | cargo-nextest (runner), `proptest` (domain invariants)                                    |
| TypeScript bindings         | `ts-rs` 12 — `#[derive(TS)]` on DTOs generates files committed under `src/bindings/`      |
| Frontend tests              | Vitest 4 + jsdom, driving React through `react-dom/client` and `act` (no Testing Library) |
| Frontend linter / formatter | oxlint 1.76 (`.oxlintrc.json`) / oxfmt 0.61 (`.oxfmtrc.json`)                             |
| Dead-code gate              | knip (`knip.json`)                                                                        |
| Logging                     | `tracing` + `tracing-appender`, local file output, default level INFO                     |
| Toolchain pinning           | mise — Rust 1.97.1 (rustfmt, clippy, llvm-tools-preview), Node 24.18.0, pnpm 11.17.0      |

**These are fixed choices, not defaults.** Do not swap a library out without changing the design
first.

## Setup

```sh
mise install                  # toolchain + cargo-nextest + cargo-llvm-cov
pnpm install --frozen-lockfile
mise run fetch-pdfium         # required before any Rust test run
```

`mise run fetch-pdfium` downloads the PDFium shared library pinned in
`scripts/pdfium-version.txt` and checks it against `scripts/pdfium-checksums.txt`. **A checksum
mismatch fails the build.** The binary lands in `src-tauri/resources/pdfium/` and is never
committed — it is large, platform-specific, and reproducibly re-fetched. CI runs the same script.

## Commands

| Command                    | What it does                                                                        |
| -------------------------- | ----------------------------------------------------------------------------------- |
| `pnpm tauri dev`           | Run the app (starts Vite on 1420)                                                   |
| `pnpm dev`                 | Frontend only, no Rust                                                              |
| `pnpm build`               | `tsc` + `vite build`                                                                |
| `mise run lint`            | `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm oxlint`, `pnpm knip` |
| `mise run fmt`             | `cargo fmt --all --check`, `pnpm oxfmt --check`                                     |
| `mise run test`            | `cargo nextest run --workspace`, `pnpm vitest run`                                  |
| `pnpm exec tsc --noEmit`   | Frontend type check (a separate CI gate — oxlint cannot see type errors)            |
| `cargo build --workspace`  | Compile both crates                                                                 |
| `scripts/make-fixtures.sh` | Regenerate the git-ignored PDF test fixtures                                        |

## Testing policy

**TDD is mandatory: write the failing test first, then the smallest implementation that passes
it.** Tests are not counted against the PR size limit, so there is no budget reason to skip them.

Each layer is tested at the level where its mistakes actually live:

- **`domain`** — unit tests plus `proptest` for the invariants that unit tests cannot cover by
  example: the multiset of slots is preserved across operations, redo after undo is the identity,
  and automatic regrouping fires only for a contiguous, monotonically increasing run.
- **`application`** — use cases run against `FakePdfEngine`, an in-memory deterministic second
  implementation of the port. It exists to prove the port is replaceable at all; a port with one
  implementation is not a port.
- **`infrastructure`** — integration tests against real PDF fixtures under
  `crates/core/tests/`: multi-page, encrypted, corrupt, mixed page sizes, image formats. These
  need the PDFium binary in place.
- **Merged output** — **never compared byte for byte.** The output is probed again to check page
  count and per-page dimensions, and each page is rasterized and compared with a perceptual hash
  (`crates/core/tests/support/phash.rs`). Byte comparison would pin PDFium's exact writer output
  and break on every upgrade.
- **`presentation`** — command bodies are extracted into `*_inner(&AppState, …)` functions so
  `src-tauri/tests/commands.rs` can drive them without a webview.
- **Frontend** — Vitest over jsdom: grouping derivation, drop-position maths, keyboard shortcut
  resolution, virtualization visible-range behaviour, the thumbnail LRU cache (including that
  eviction revokes the object URL), and store transitions. Pure logic is deliberately kept out of
  components (`src/lib/*.ts`) so it can be tested without a DOM.

## Conventions

- **Everything in this repository is written in English** — code comments, identifiers, commit
  messages, and the title and body of every pull request and issue. Comments explain _why_, not
  _what_. The authoritative design documents live in Notion and are in Japanese; that is outside
  this repository and stays as it is.
- **Layer boundaries are enforced by review, not by a lint.** `domain` stays free of IO, async and
  external crates; dependencies flow presentation → application → domain and infrastructure →
  domain, never back. See [`architecture.md`](architecture.md).
- `crates/core` is `#![forbid(unsafe_code)]`; both crates deny all Clippy lints.
- DTO changes regenerate `src/bindings/**` via `ts-rs`; **commit the regenerated files** in the
  same PR, otherwise the type-check gate catches the drift for you.

## PR rules

- **One PR = one task = one GitHub issue.** Production code stays under 800 lines; test code is
  not counted.
- **Pull requests and issues are written in English**, title and body alike — see
  [Conventions](#conventions).
- **Commit messages are a single line.** No body, no trailers, and **no AI attribution of any
  kind**.
- **Every gate must be green before merge**: `cargo clippy -- -D warnings`, `cargo fmt --check`,
  `cargo nextest run`, `pnpm exec tsc --noEmit`, `pnpm oxlint`, `pnpm oxfmt --check`, `pnpm knip`,
  `pnpm vitest run`.
- **Never commit the PDFium binary** or anything else under `src-tauri/resources/pdfium/`.
- `docs/superpowers/` is git-ignored. Anything written there does not ship — put documentation
  that should ship in `docs/architecture.md` or this file.

## CI and release

- **CI** (`.github/workflows/ci.yml`) runs two jobs: _Rust_ on macOS (fetch PDFium → clippy →
  fmt → `cargo llvm-cov nextest` → Codecov) and _Frontend_ on Ubuntu (tsc → oxlint → oxfmt →
  knip → vitest).
- **Release** (`.github/workflows/release.yml`) triggers on a `v*` tag push, or manually against
  an existing tag. It builds a macOS universal bundle and a Windows bundle with `tauri-action`,
  and uploads them to the matching GitHub release. Release notes are authored by hand; the
  workflow never edits the release body.
- On macOS the PDFium dylib is **ad-hoc signed before** Tauri seals the app, because deep signing
  does not reach a loose dylib under `Contents/Resources` and the downloaded universal library
  ships an unsigned x86_64 slice. `scripts/macos/verify-macos-bundles.sh` then verifies the
  signature of the `.app` _and_ of the `.app` mounted inside the `.dmg`.
- Missing macOS signing secrets fail the job up front, so an unsigned macOS build can never be
  published by accident. The three secrets are `APPLE_CERTIFICATE` (a base64-encoded `.p12`),
  `APPLE_CERTIFICATE_PASSWORD` and `KEYCHAIN_PASSWORD`.
- The signing certificate is **self-signed** — not issued by Apple, and the app is not notarized,
  so Gatekeeper still warns on first launch. Generate it with
  `P12_PASSWORD='…' ./scripts/macos/generate-self-signed-cert.sh`, which writes a git-ignored
  `cert-out/`. Its Common Name must start with `Developer ID Application: ` and it **must carry an
  OU**, which Tauri reads as the Team ID; without one the build fails with `certificate missing
organization unit for common name`. Full procedure, local verification recipe and failure modes:
  [`.claude/macos-code-signing.md`](../.claude/macos-code-signing.md).

## Performance measurement

Measured SLO figures, their methodology, and the items that remain unmeasured are recorded in
[`architecture.md` § Service level objectives](architecture.md#service-level-objectives). When you
add a measurement, put the number and a link to its source there, and keep measured and estimated
rows visibly distinct.
