"""Architecture diagram for pdf-tools.

Generates ``architecture.png`` in this directory: a single-image overview of
the Mac/Windows native desktop app (Tauri 2) and the three paths a merge can
take through ``compose`` (one per outbound port).

Run from this directory::

    python3 diagram.py

What is shown (matches docs/architecture.md):
  - A thin Tauri 2 desktop app: the React webview UI plus the presentation
    layer, drawn as one node carrying the app's own shield logo (no cluster
    frame). The merge plan never lives here; the UI mirrors the snapshot Rust
    returns.
  - A pure Rust core crate (pdf-tools-core): the engine and the three compose
    paths fanning out from it. The outbound ports are shown as edge labels
    (PdfEngine port / ImageDecoder port) rather than as nodes:
      * PDF-backed slots: the page is copied and the slot's rotation is
        written as the page's /Rotate attribute, so the content stream is
        never rewritten.
      * Image-backed slots, eligible JPEG: the original DCTDecode stream is
        embedded untouched.
      * Image-backed slots, everything else: decoded and re-embedded under a
        200 DPI cap, fitted to the plan's dominant page size.
  - Thumbnails and merge progress return to the webview over dotted,
    non-constraining edges.
  - The app is bundled as a native binary for macOS and Windows.
"""

from diagrams import Cluster, Diagram, Edge
from diagrams.custom import Custom
from diagrams.onprem.client import Users
from diagrams.programming.flowchart import Document, PredefinedProcess
from diagrams.programming.language import Rust

# Logos staged in icons/ — kept locally so the diagram renders without network
# access. logo.png is this project's own mark; the platform icons match the
# ones the sibling simple-archiver diagram uses.
ICON_LOGO = "icons/logo.png"
ICON_APPLE = "icons/apple.png"
ICON_WINDOWS = "icons/windows.png"
ICON_PDF = "icons/pdf.png"

graph_attr = {
    "fontsize": "18",
    "splines": "spline",
    "pad": "0.5",
    "nodesep": "0.6",
    "ranksep": "1.3",
}


with Diagram(
    "PDF Tools — Mac/Windows desktop architecture",
    filename="architecture",
    show=False,
    direction="LR",
    outformat="png",
    graph_attr=graph_attr,
):
    # External actor.
    user = Users("End user\n(drag & drop\nPDFs / jpg · png · gif)")

    # Presentation: the whole Tauri 2 desktop app as one node — the React
    # webview UI plus the Tauri command layer (no cluster frame).
    app = Custom("PDF Tools\n(Tauri 2 · React webview)", ICON_LOGO)

    # Output sink on the local filesystem; all three paths converge here.
    # The label is broken across three lines rather than two: this node sits
    # at the right edge of the drawing, and a wider caption is clipped there.
    out = Custom(
        "merged.pdf\n(one file ·\npage order = plan order)", ICON_PDF
    )

    # Pure Rust core crate: the engine plus the three compose paths.
    with Cluster("pdf-tools-core (Rust crate)"):
        # The engine carries the Rust logo only (no caption).
        engine = Rust("")

        # Path 1 — PdfEngine port: the source page is copied and the slot's
        # rotation becomes the page's /Rotate attribute.
        with Cluster("compose · PDF-backed slots"):
            copy_page = Document("Copy page\n(+ /Rotate attribute)")

        # Paths 2 and 3 — ImageDecoder port: an eligible JPEG keeps its own
        # stream; every other image is decoded and re-embedded under the cap.
        with Cluster("compose · image-backed slots"):
            passthrough = Document(
                "JPEG passthrough\n(original DCTDecode stream)"
            )
            reembed = PredefinedProcess(
                "Decode + re-embed\n(200 DPI cap, fitted to the\n"
                "dominant page size)"
            )

    # Native distribution targets.
    with Cluster("Native targets (Tauri bundle)"):
        macos = Custom("macOS\n(.dmg)", ICON_APPLE)
        windows = Custom("Windows\n(.exe / .msi)", ICON_WINDOWS)

    # ────────────────────────────── Edges ──────────────────────────────

    # Request spine: user -> app -> engine. High weight + thick stroke pins
    # these onto the same horizontal rank.
    user >> Edge(label="drag & drop", penwidth="2", weight="10") >> app
    app >> Edge(
        label="add_sources / reorder /\nrotate_slots / remove_slots\n"
        "→ PlanSnapshot",
        penwidth="2",
        weight="10",
    ) >> engine

    # The two outbound ports are shown only as edge labels — one per path.
    engine >> Edge(
        label="PdfEngine port\n(probe · compose)", style="dashed"
    ) >> copy_page
    engine >> Edge(
        label="ImageDecoder port\n(eligible JPEG)", style="dashed"
    ) >> passthrough
    engine >> Edge(
        label="ImageDecoder port\n(every other image)", style="dashed"
    ) >> reembed

    # All three paths converge on one output file. Only the first edge is
    # labelled; repeating "write" three times into one node adds nothing.
    copy_page >> Edge(label="write", penwidth="2") >> out
    passthrough >> Edge(penwidth="2") >> out
    reembed >> Edge(penwidth="2") >> out

    # Thumbnails and progress stream back to the webview. The plan itself
    # never leaves Rust.
    engine >> Edge(
        label="rasterize_slot\nPNG bytes → LRU blob cache",
        style="dotted",
        constraint="false",
    ) >> app
    engine >> Edge(
        label="compose-progress\nevents",
        style="dotted",
        constraint="false",
    ) >> app

    # The same app is packaged as a native binary for each desktop OS.
    app >> Edge(
        label="native bundle", style="dotted", constraint="false"
    ) >> macos
    app >> Edge(style="dotted", constraint="false") >> windows
