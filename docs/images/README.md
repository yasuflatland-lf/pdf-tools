# Architecture Diagram

Source for the architecture diagram of `pdf-tools`. The script `diagram.py` uses the
[diagrams](https://diagrams.mingrammer.com/) Python library (which renders via
[Graphviz](https://graphviz.gitlab.io/)) and emits `architecture.png` in this directory.

## Prerequisites

- Python 3.6+
- [Graphviz](https://graphviz.gitlab.io/download/)
- [Diagrams](https://diagrams.mingrammer.com/docs/getting-started/installation#quick-start)

On macOS:

```bash
brew install graphviz
pip3 install diagrams
```

## Generate

From this directory:

```bash
python3 diagram.py
```

The script writes `architecture.png` next to itself. Re-run it after every change to
`diagram.py`; the output is regenerable, so **do not hand-edit the PNG**.

## Icons

`icons/` holds PNGs so the diagram renders without network access.

- `logo.png` is this project's own shield mark, rasterized from `public/logo.svg`:

  ```bash
  rsvg-convert -w 512 -h 512 -a ../../public/logo.svg -o icons/logo.png
  ```

- `apple.png` and `windows.png` are copies of the platform icons used by the sibling project
  [simple-archiver](https://github.com/yasuflatland-lf/simple-archiver), so the two diagrams
  render the same artwork at the same size.

- `pdf.png` marks the merged output. It was rasterized from a PDF file icon downloaded from
  [SVG Repo](https://www.svgrepo.com/):

  ```bash
  rsvg-convert -w 512 -h 512 -a pdf-file-svgrepo-com.svg -o icons/pdf.png
  ```

The Rust logo is not staged here — it ships with the `diagrams` library as
`diagrams.programming.language.Rust`.

## What the diagram shows

- **PDF Tools (Tauri 2 · React webview)** — one node for the whole desktop app: the webview UI
  plus the presentation layer (Tauri commands, DTOs, the `PlanSnapshot` every command returns).
  Rust is the single source of truth; the UI replaces its store contents with the snapshot rather
  than deriving a plan of its own.
- **pdf-tools-core (Rust crate)** — the engine (Rust logo, no caption) drives **three compose
  paths**. The outbound ports appear as edge labels (`PdfEngine port` / `ImageDecoder port`)
  rather than as nodes:
  - **PDF-backed slots** — the source page is copied and the slot's rotation is written as the
    page's `/Rotate` attribute, so the content stream is never rewritten.
  - **Image-backed slots, eligible JPEG** — the original `DCTDecode` stream is embedded untouched.
  - **Image-backed slots, everything else** — decoded and re-embedded under a 200 DPI cap for the
    occupied page area, fitted to the plan's dominant page size.
  - All three converge on a single `merged.pdf` whose page order is the plan's order.
- **The return path** — `rasterize_slot` sends thumbnails back as raw PNG bytes into the
  frontend's LRU blob cache, and `compose-progress` events report merge progress. Both are drawn
  dotted, because neither carries the plan.
- **Native targets** — the same app is bundled for **macOS** (`.dmg`) and **Windows**
  (`.exe` / `.msi`).

The pure `domain` layer, the undo/redo stacks and the grouping rules are not drawn — see
[`../architecture.md`](../architecture.md) for the full layer boundaries and
[`../development.md`](../development.md) for the tech stack.
