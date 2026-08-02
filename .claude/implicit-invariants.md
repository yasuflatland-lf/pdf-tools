# Implicit invariants (L3)

> Doc layers: L1 = [`/CLAUDE.md`](../CLAUDE.md) / L2 = [`docs/architecture.md`](../docs/architecture.md) / L3 = this file.

Four properties the app relies on that no type, assertion or test at the point of use enforces.
Each one holds because something elsewhere upholds it. This file records what that something is,
what breaks if it stops, and how far each property was actually checked. The summary a contributor
needs lives in [`docs/architecture.md`](../docs/architecture.md) under "Invariants nothing
enforces"; this file is the working behind it.

## 1. Slot ids in a plan are unique

**What holds.** No two `PageSlot`s in a `MergePlan` share a `SlotId`.

**What upholds it.** `IdSequence::next_slot` (`crates/core/src/domain/ids.rs:23-27`) increments a
`u64` on every call, and it is the only producer of a `SlotId`. `PlanSession` owns the one sequence
and deliberately does not snapshot it, so undo cannot rewind it into reissuing an id
(`crates/core/src/application/session.rs:16-17`).

**What `MergePlan` does about it.** Nothing. `MergePlan::new` takes a `Vec<PageSlot>` and stores it.

**What breaks if it stops.** Every operation that names slots by id acts on all the copies:

```
plan = [ #7(src0,p0), #7(src0,p1), #8(src1,p0) ]

remove(plan, [SlotId(7)])     -> [ #8 ]            one id named, two pages gone
rotate(plan, [SlotId(7)], 1)  -> both #7 turned    one id named, two pages turned
```

`SlotTarget::resolve` (`crates/core/src/application/rasterize_slot.rs:36-50`) uses `find`, so a
thumbnail would always show the first copy, whichever the user clicked.

**How far it was checked.** A bounded model of the session was run over 14,641 operation sequences
(depth 4 over add / remove / remove_source / rotate / reorder / undo / redo) and no reachable state
had a duplicate id. The failure above was produced by constructing the duplicate directly, which is
only possible because `MergePlan::new` accepts it.

**If it ever needs enforcing.** A `debug_assert!` in `MergePlan::new` is the cheap version and costs
nothing in release. Making the type reject duplicates outright would make `MergePlan::new` fallible,
which every undo/redo install path currently relies on being infallible
(`crates/core/src/domain/document.rs:11-13`).

## 2. A failed source owns no slot

**What holds.** If `SourceFile::status()` is not `Ready`, no slot in the plan names it.

**What upholds it.** `AddSources::execute` (`crates/core/src/application/add_sources.rs:35-63`).
Every failure arm builds a `SourceFile::failed` and pushes nothing to `new_slots`; only the two `Ok`
arms extend it. `SourceFile::failed` itself stores a zero page count and no page sizes; the count is
exposed through `page_count()` rather than as a field, so a failed source has nothing a slot could
point at.

**What `MergeDocument` does about it.** Nothing. Its constructor establishes the other direction —
every slot names a listed source — and says so. This direction is not part of its contract.

**What breaks if it stops.** `Compose::execute` already assumes it does not:

```rust
            .filter(|slot| document.source_of(slot).status() == SourceStatus::Ready)
```

That filter is unreachable defence in the current code. Its test
(`a_document_with_only_unusable_sources_reports_the_real_situation`) constructs the state by hand,
which is the clearest evidence the type permits what the app never produces. Without the filter, a
merge would ask PDFium for pages of an encrypted file.

**How far it was checked.** The same 14,641 sequences. No reachable state had a failed source owning
a slot.

**If it ever needs enforcing.** Splitting `SourceFile` into a ready variant carrying page sizes and
a failed variant carrying only a status makes the state unrepresentable, and removes the
`debug_assert!` at `crates/core/src/domain/source.rs:114` along with it. That is a type change with
reach into every construction site.

## 3. A grouped source folds into exactly one card

**What holds.** When Rust reports a source as `grouped`, the frontend draws its pages as one card;
when it reports `ungrouped`, it draws one card per page.

**What upholds it.** Two independent implementations of one rule.
`can_regroup` (`crates/core/src/domain/grouping.rs:7-28`) decides contiguity, strict page ascent and
uniform rotation. `groupContiguous` (`src/lib/grouping.ts:33-57`) folds adjacent slots while the
source is `grouped`, the source id matches, and the rotation matches the run's first slot. The
second and third of those conditions are already implied by the first whenever Rust said `grouped` —
which is what makes the repetition harmless rather than a second opinion.

**What checks it.** Nothing at either boundary. The snapshot carries a per-source
grouped/ungrouped flag (`GroupingDto`) and not the grouping itself, so the frontend has to
reconstruct the runs.

**What breaks if it stops.** Changing one side alone. Relaxing Rust's ascent from strict to
non-decreasing, for instance, would make Rust report `grouped` for a run holding one page twice
while the frontend still folds it, and a collapsed card would claim a page count its run does not
have. That is exactly the failure `docs/architecture.md:129-131` explains the strict ascent exists
to prevent — but the explanation lives only on the Rust side of the rule.

**How far it was checked.** Every plan of up to 4 slots over 2 sources, 3 page indices and 2
rotations — 22,620 plans. For each: cards tile the plan with no gap or overlap; a grouped source
yields exactly one card; an ungrouped source yields only single-page cards. No violation.

**If it ever needs enforcing.** Shipping the grouping itself in the snapshot rather than a flag
deletes `groupContiguous` and the question with it.

## 4. A drop lands where the drag preview showed it

**What holds.** Dropping card A onto card B's position produces the plan that moving A to B's index
in the card list would produce — regardless of how many pages each card stands for.

**What upholds it.** The two branches of `computeDropTarget` (`src/lib/drop-position.ts:42-44`)
matching `reorder`'s definition of `to` as an index into the sequence remaining after the span is
lifted out (`crates/core/src/domain/operations.rs:44-47`). A backward move needs no adjustment
because the lifted span sits after the target; a forward move subtracts the span's own length,
which is what the `over.start + over.pageCount - active.pageCount` arm is doing.

**What checks it.** `src/lib/__tests__/drop-position.test.ts` checks the arithmetic against
hand-written expectations. Nothing composes it with `reorder` and compares the result to the card
list the user saw.

**What breaks if it stops.** Cards of unequal weight land off by the difference. A single-page card
dropped onto a collapsed 12-page group would miss by 11 positions, silently, with no error anywhere.

**How far it was checked.** 90,480 card layouts — the same plan space as above, crossed with all
four combinations of the two sources being expanded or collapsed — with every legal (active, over)
pair on each. The composition of `computeDropTarget` and `reorder` equalled
`arrayMove(cards, active, over)` in every case.

## Scope of the checks

All figures above come from a bounded model: the Rust and TypeScript implementations transcribed
literally into one executable model and run over the full state space of small plans. Bounded means
bounded — **no claim is made that a counterexample cannot exist at 5 slots or at 3 sources.** What
the checks do establish is that none of these four properties fails for a reason that shows up
small, which is where this class of bug normally shows up first.

The value-object algebra was checked separately with Z3 rather than by enumeration: `PageSize`'s
lattice-cell equality is a genuine equivalence relation, `Rotation` is a cyclic group of order four,
`natural_cmp` is a total order, and `dominant_page_size` does not depend on the `HashMap` iteration
order it is computed from.
