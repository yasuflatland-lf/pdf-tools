# Numeric precision (L3)

> Doc layers: L1 = [`/CLAUDE.md`](../CLAUDE.md) / L2 = [`docs/architecture.md`](../docs/architecture.md) / L3 = this file.

Why page geometry is `f32`, which alternatives were rejected and on what grounds, which `f32`
hazards are real and where each is already handled, and what to change first if precision ever does
become a problem. The summary a contributor needs lives in
[`docs/architecture.md`](../docs/architecture.md) under "Numeric representation"; this file is the
working behind it.

## The one type, and its whole lifetime

`PageSize` holds two `f32` fields and nothing widens them anywhere along the path:

| Stage        | Where                                                | Type                          |
| ------------ | ---------------------------------------------------- | ----------------------------- |
| Read out     | `crates/core/src/infrastructure/pdfium/rasterize.rs` | `pdf_page.width().value: f32` |
| Held         | `crates/core/src/domain/geometry.rs`                 | `PageSize { f32, f32 }`       |
| Fitted       | `crates/core/src/infrastructure/pdfium/compose.rs`   | `Placement { f32 × 4 }`       |
| Written back | `crates/core/src/infrastructure/pdfium/compose.rs`   | `PdfPoints::new(f32)`         |
| Crosses IPC  | never                                                | —                             |

The last row is load-bearing. `grep -rn "f32\|f64" src-tauri/src` returns nothing, and every number
in `src/bindings/**` is an integer — slot ids, page indices, quarter turns, byte counts. **A page
dimension never leaves Rust**, so no question about JSON round-tripping, about the frontend's
`number` being an IEEE 754 double, or about narrowing on the way back arises at all.

## Why not arbitrary precision (`bigdecimal`, `rust_decimal`)

**The decisive reason is the layering rule.** `domain` is pure — no IO, no async, no external
crates. `PageSize` lives in `domain`. An arbitrary-precision decimal type is an external crate, so
it cannot go where the value is held. Nothing else needs to be argued to settle the question, but
four further reasons hold independently:

1. **Nothing downstream can carry the precision.** `pdfium-render`'s `PdfPoints` is
   `pub struct PdfPoints { pub value: f32 }` (`points.rs:19`), and `compose` hands every dimension
   back through `PdfPoints::new(...)`. Whatever precision the domain computed narrows to single at
   the port boundary. At the format level, ISO 32000-1:2008 Annex C.1 gives PDF's architectural
   limit for real numbers as approximately ±3.403 × 10³⁸ with approximately five significant
   decimal digits — the bounds of an IEEE 754 single.
2. **Nothing upstream produces it.** The dimensions are measured by PDFium and arrive as `f32`.
   Precision cannot be recovered that was never present in the input.
3. **There is no accumulation chain for error to grow along.** Arbitrary precision earns its cost
   where a value is fed back into its own computation thousands of times. Here a rotation is a
   `/Rotate` page attribute rather than a coordinate transform, so it performs no arithmetic at
   all, and the only arithmetic performed on a `PageSize` is one division and two multiplications
   in `placement` (`compose.rs`), evaluated once per slot from the source values and never fed
   back. Error has nowhere to compound.
4. **It would not fix the thing that looks like a precision problem.** See "Same size is a
   tolerance question" below: two A4 pages from different producers genuinely measure 595.276 and
   595.2756 pt, and exact decimal arithmetic calls those different just as exact float comparison
   does.

Cost is the least of it, but it is real: every value becomes a heap allocation, against a
merge-100-pages-in-10-s objective whose margin on the raster path is already thin.

## Why not `f64`

Not wrong, merely pointless. It buys precision at exactly the boundary that `PdfPoints` discards,
and it widens the public signature of every `domain` accessor to pay for it. It is also not the
right response to the one problem it would actually solve — see step 2 of the escalation order
below, which reaches for `f64` locally, inside one function, without changing a stored type.

## What single precision actually costs here

| Quantity                                       | Points             |
| ---------------------------------------------- | ------------------ |
| A4 portrait width, as written in `A4_PORTRAIT` | 595.276            |
| the same value, as actually stored in `f32`    | 595.2760009765625  |
| that rounding error                            | 9.8 × 10⁻⁷         |
| one ULP at that magnitude                      | 1.22 × 10⁻⁴ (2⁻¹³) |
| one output pixel at the 200 DPI embed cap      | 0.36 (72 / 200)    |
| one `SizeClass` lattice cell (`LATTICE_PT`)    | 1.0                |

The ordering is what matters, and it spans four orders of magnitude in each step: **one ULP ≪ one
pixel ≪ one lattice cell.** A pixel at the embed cap is about 2 950 ULPs wide; a lattice cell is
exactly 8 192 of them. The representable spacing of the type is nowhere near being the limiting
factor on either what the merge draws or what the domain treats as the same size.

## The `f32` hazards that are real, and where each is handled

Floats carry four traps. Three are answered by a deliberate decision in the code; the fourth cannot
arise here at all. None of them is open.

1. **Float comparison is too fine-grained to mean "the same page size", and `NaN` breaks the
   equivalence relation outright.** `PageSize::new` rejects any dimension that is not positive and
   finite (`geometry.rs`), so `NaN` never enters a `PageSize` in the first place, and `PartialEq`
   is hand-written over `size_class()` rather than derived, so equality is lattice-cell equality.
2. **`f32` implements neither `Eq` nor `Hash` nor `Ord`, so it cannot key a map or be sorted.**
   `SizeClass` is two `i32` cell indices and derives `Eq + Hash`, which is what lets
   `MergeDocument::dominant_page_size` fold its slots into a `HashMap<SizeClass, _>`
   (`document.rs`). Quantising to integers is what makes the key legal, not a workaround for one.
3. **Serialization drift across a language boundary.** Not applicable, and structurally so: no
   float crosses IPC (see the table above).
4. **Assertions failing on a trailing digit.** The same hand-written `PartialEq` covers this:
   `assert_eq!` on two `PageSize` values compares lattice cells, so a test cannot fail on a
   difference no reader of the output could see. The `proptest` generator in `document.rs` draws
   dimensions from `1.0..2000.0` and needs no epsilon for the same reason.

## Same size is a tolerance question, not a precision question

This is the point most easily missed, because it looks like a precision problem and is not.

Two PDFs that both call themselves A4 will report 595.276 pt and 595.2756 pt when they were
produced by different tools. The two values are different real numbers. No numeric type makes them
equal — `f64` does not, and an exact decimal type does not either; exact arithmetic makes the
disagreement _more_ faithfully represented, not less. What is needed is a statement of how close
counts as the same, and that is a modelling decision, not a representation one.

`SizeClass` is that statement: round each dimension onto a one-point lattice and compare cells.
`LATTICE_PT` is therefore the knob for "how close is the same", and it is the thing to reach for
when a classification looks wrong. Changing the numeric type would not move it.

## If precision ever does become a problem

In this order. Each step is cheaper than the next, and steps 1–3 change no public type.

1. **Prove it.** Name the value, the computed and the expected figure, and the size of the
   divergence — then compare that against 0.36 pt, one output pixel at the embed cap. A divergence
   below that cannot be the cause of anything a user reported.
2. **If it is accumulation** — a future feature summing many page dimensions, which no current code
   does — accumulate in `f64` inside that one function and narrow on return. Local change, no
   stored type touched.
3. **If it is classification** — two sizes that should be one landing in different cells, or two
   that should differ sharing one — change `LATTICE_PT`. That is the parameter that decides it.
4. **Only if 1–3 are all exhausted** does the stored type come into question, and the candidate
   then is `f64`, which would need `PdfPoints` to stop being the last hop. Arbitrary precision
   stays barred from `domain` by the layering rule regardless of the finding.
