# 05 — Research questions and roadmap

Phase 0 is *research, on paper* — the expensive mistakes in language design
are the ones implemented before they were designed. Every phase has an exit
gate and a kill criterion; hitting a kill criterion means redesign, not
denial.

## Phase 0 — Research spikes (~3 months, papers and prototypes ≤ 500 LOC)

### RQ1 — Typed actors: the moat question
Can supervised, typed message-passing be made *less* ceremonial than
Elixir's GenServer while remaining sound?
- Study deeply: Gleam's typed-OTP subset, Akka Typed (what made it
  awkward), Pony's reference capabilities (what made it powerful and what
  made it unlearnable), Erlang behaviours, session-types literature
  (know why it stays academic), Swift actors + structured concurrency.
- Deliverable: a written design for `process`/`send`/`receive`/`supervise`
  with typed process references and typed mailboxes, exercised in sample
  programs — including the hard cases: protocol evolution (adding a
  message variant), timeouts, monitors, and a supervision tree restart.
- **Gate:** the supervised typed echo server sample reads better than its
  GenServer equivalent. **Kill criterion:** if it needs more annotations
  or concepts than GenServer, the actor design is wrong — iterate before
  anything else proceeds.

### RQ2 — Runtime feasibility: can a small team build it?
- Study: Go's scheduler and preemption (async preemption via signals),
  BEAM's per-process heaps and reduction counting, Pony's runtime
  (ORCA GC, work stealing), Tokio (what cooperative scheduling costs in
  ergonomics).
- Decide the scope ladder deliberately: (a) cooperative + compiler-inserted
  safepoints → (b) preemption; (i) shared immutable heap + per-process
  nurseries → (ii) full isolation. Where on each ladder does v0.1 land?
- Scope now explicitly includes the P12 memory model (doc 06): per-process
  GC design, escape analysis feasibility, scoped-arena escape checking,
  and the copy-vs-share policy for message sends / large binaries.
- Deliverable: a feasibility memo with a scheduler/GC design sketch and a
  ≤ 500-line native proof-of-concept (spawn 1M green processes, message
  ring benchmark, one hostile busy-loop process that cannot starve the
  system).
- **Gate:** PoC demonstrates isolation + fairness on one machine.
  **Kill criterion:** if the memo concludes the runtime needs > ~2
  engineer-years to v0.1 quality, cut scope via the ladder or reconsider
  hosting semantics on an existing runtime (which reopens RQ4).

### RQ3 — Samples-first surface design
Write the **10 canonical programs** *before* the grammar exists; they are
the spec's test suite and every future feature's audition:
1. HTTP JSON service with routing + graceful shutdown
2. Supervised worker pool chewing a queue
3. Typed echo server (the RQ1 gate program)
4. JSON decode/encode with schema evolution (P10)
5. Concurrent web scraper with timeouts, retries, and backpressure
6. CLI tool (args, files, exit codes — the boring path must be pleasant)
7. State machine with exhaustive matching (order lifecycle)
8. Error-handling showcase: fallible pipeline, `?`, recovery, crash+restart
9. Test file: table-driven tests + a property test for program 7
10. A 3-module program exercising public-boundary annotations (P3)
11. Hot-path buffer pipeline in Tier 0 then Tier 1 (P12) — the delta must
    be local and signatures must not change
- Written in 2–3 candidate surface syntaxes; reviewed for the P9 gate
  ("would a Python dev read this cold?").
- **Gate:** one syntax direction chosen, with the sample suite as ADR-001.

### RQ4 — The strategic interop bet
Evaluate against doc 03's assumption 3 (ecosystem cold-start): (a) C-ABI
FFI only + fat stdlib, (b) secondary compile-to-BEAM target (courts the
Elixir wedge; risks splitting runtime semantics), (c) WASM component
model (courts polyglot embedding; immature), (d) Roc-style embedding
platforms. Deliverable: ADR-002 choosing exactly one *primary* bet for
v0.x. **Guard:** whatever is chosen must not violate P8 (single binary).

### RQ5 — AI-era lens, cheap experiments
Measure, don't assume: give current models the sample programs' specs and
evaluate generated code against the candidate syntaxes for correctness,
diff-reviewability, and token cost vs Go/TS equivalents. Feed findings
into P2/P3/P9 decisions. (Low effort; tailwind validation per doc 03.)

## Phase 1 — Spec 0.1 + walking skeleton (~3 months)
Grammar + prose spec for the core (small enough to read in an evening —
P2's budget made verifiable); tree-walking interpreter running samples
1–8; `fmt` exists from the *first month* (P6 — the formatter is part of
the language, not tooling that comes later).
**Gate:** all 10 samples parse, format canonically, and run (slowly).

## Phase 2 — Type checker (~3–4 months)
Module-local inference (P3), structural records, unions + exhaustive
narrowing, generics with simple bounds; diagnostics designed as
first-class output (human prose + machine-readable JSON from day one —
the LSP and the AI lens both consume them).
**Gate:** samples type-check; deliberately-broken variants produce
diagnostics a newcomer can act on. Compile-speed benchmark harness (P7)
starts running in CI *now*, not in Phase 4.

## Phase 3 — Runtime + native backend (~6 months, overlapping)
Implement the RQ2 decision; backend candidates: Cranelift (fast compiles,
fits P7), LLVM (perf ceiling, slow), or compile-to-Zig/C as a stopgap.
P7's budget picks the default; nothing forbids a second optimizing
backend later.
**Gate:** samples run as single static binaries; the RQ2 hostile-process
benchmark holds; supervision restart sample survives a kill -9 of a
worker process.

## Phase 4 — Tooling completion (~3 months, overlapping)
LSP (hover, completion, rename, references — powered by the same checker),
`test` runner with the table/property idioms from sample 9, `doc`, basic
`pkg` with lockfiles. **Gate:** the language can be developed *in itself*
comfortably by a stranger following only the README.

## Working method

- **ADRs** in `docs/decisions/`, numbered, one page each; every pillar
  trade-off (doc 04) that gets resolved cites its evidence.
- **Samples are the constitution:** any proposed feature must first
  improve at least one sample without degrading another.
- **Name and branding:** deliberately deferred until RQ3's syntax gate —
  naming before identity invites bikeshedding.
- **Cadence:** this repo is the single source of truth; research notes
  land in `docs/notes/` as they happen, summarized into the numbered docs.

## Immediate next actions

1. RQ1 reading list → notes in `docs/notes/rq1-*.md`.
2. Draft samples 1, 3, and 8 in two candidate syntaxes (start of RQ3).
3. RQ2 reading on Go's async preemption + Pony's ORCA, before any
   runtime opinions harden.
