# 06 — The memory debate

## The stance under examination

> Developers shouldn't have to worry about handling memory if they don't
> want to — but should have the option to choose to do so if needed.

This document stress-tests that stance against the niche (doc 03), the
pillars (doc 04), and the field's prior art, then turns it into a concrete
tiered model. Verdict up front: **the stance survives, but only with a
discipline attached** — opt-in control must change *performance*, never
*soundness*, and must never fork the ecosystem into two dialects.

## The design space

| Model | Exemplars | Worry-free? | Control? | Fit with our pillars |
|---|---|---|---|---|
| Manual | C, Zig | No | Total | Violates P1 (soundness) outright |
| Ownership/borrowing | Rust | No — the borrow checker *is* the worry | Total, safe | Fights P9 (readability) and the AI lens: LLMs demonstrably mishandle lifetimes and reach for `unsafe`/`unwrap` |
| Reference counting | Swift (ARC), Nim (ORC) | Mostly | Middling | Predictable, but cycles + hidden RC traffic; Swift's later ownership additions show RC alone wasn't enough |
| Tracing GC, global heap | Go, JVM, C# | Yes | Escape hatches | Global pauses fight tail latency; Go's arena experiment was abandoned — bolting arenas onto a global-heap GC got stuck |
| Tracing GC, per-process heaps | BEAM (Erlang/Elixir) | Yes | Almost none | Isolation makes GC *local*: pauses are per-process and micro-scale; the model our runtime already commits to (P4) |
| Compile-time RC + reuse | Koka (Perceus), Roc | Yes | Little | Elegant for pure FP cores; young; pairs poorly with long-lived mutable process state |
| Regions/arenas as language design | Vale, Verona (research), Odin-style allocator discipline | Yes-ish | Structured | The most promising *shape* for our opt-in tier: control with a lifetime story, no borrow checker |

## What the actor model changes about this debate

Most "GC vs manual" arguing assumes one big shared heap. Our runtime
doesn't have one (P4): processes own small isolated heaps and share
nothing mutable. That reshapes the trade-offs in our favor:

1. **The default tier gets cheap.** Collecting a 200 KB process heap takes
   microseconds and pauses only that process — the BEAM has demonstrated
   this profile in production telecom/web workloads for decades. The
   strongest objection to "worry-free by default" (global pause spikes)
   mostly dissolves.
2. **A natural region already exists: the process.** A process's heap *is*
   an arena whose lifetime is the process. "Spawn a short-lived process,
   let it die, heap vanishes" is idiomatic memory control that requires no
   new concepts — Erlang programmers have used processes-as-arenas
   forever, without calling it that.
3. **Messages are the cost center.** With isolated heaps, the real memory
   decisions concentrate at *copy vs share* on message sends. That is
   where our opt-in control has to earn its keep (large binaries/buffers),
   not in general allocation.

## Prior-art warnings for the opt-in tier

- **D's `@nogc`** split the ecosystem: libraries had to pick a side, and
  the standard library itself became partially unusable from `@nogc`
  code. *Lesson:* opt-in control must not create a function-coloring
  problem or a second stdlib.
- **Go's arena experiment** (2022–23) was shelved indefinitely: arenas
  interacted badly with the rest of the language and its API surface
  leaked everywhere. *Lesson:* memory control retrofitted as a library
  fights the language; design the seams now even though we build them
  later.
- **Nim's** ARC→ORC journey shows churn in the *default* is very costly;
  pick the default model early and keep it stable.
- **Swift** is the encouraging case: worry-free default (ARC) with later,
  opt-in ownership annotations and noncopyable types for the hot 5% —
  adopted without forking the ecosystem. Closest existing realization of
  the stance, minus our per-process advantage.

## The model: three tiers, one rule

**The rule: moving down a tier changes performance and predictability,
never memory safety — except inside `unsafe`, which exists only at the FFI
edge (P1) and is quarantined there.**

### Tier 0 — automatic (the default; target: 95% of all code)
Per-process GC'd heaps, escape analysis for stack allocation, value
semantics for small data. No annotations, no lifetimes, no `delete`.
A Python developer reads and writes this tier with zero new concepts.

### Tier 1 — structured control (opt-in, still 100% safe)
For hot paths, large buffers, and tail-latency work — control expressed
through *lifetimes the program already has*, not through a borrow checker:

- **Process-as-arena idiom, made official:** spawn-for-scratch-work is
  documented, cheap, and the first tool reached for.
- **Scoped arenas:** `arena { ... }` allocates within a lexical region
  freed at once on exit; escape is a compile-time error (region typing in
  its simplest form — no general lifetime annotations).
- **Value/inline types:** declare a type unboxed; arrays of values, not
  pointers.
- **Shared immutable buffers:** large binaries/blobs are refcounted and
  shared across process boundaries instead of copied (the BEAM's binary
  heap, typed); slices/views carry runtime bounds.
- **Capacity and pooling primitives** in the stdlib (preallocated pools,
  reusable buffers) with safe APIs.

Tier-1 constructs are ordinary expressions: a function using an arena
internally is *invisible* to its callers — no coloring, no split
ecosystem. That is the D-mistake firewall, stated as a design invariant.

### Tier 2 — `unsafe` (FFI boundary only)
Pinned memory, foreign allocation, raw pointers — inside explicit,
greppable `unsafe` blocks at the C-ABI edge, per P1. Not available as a
general programming model: application code cannot take a dependency on
manual free/use-after-free risk. This is where we deliberately stop short
of "total control"; anyone needing Zig is better served by Zig, and our
anti-personas (doc 03) already exclude them.

## What this stance costs (honesty section)

- A GC and escape analysis we must build well (folds into RQ2's runtime
  scope — the GC design and the copy-vs-share message policy are now
  explicitly part of that spike).
- Peak throughput below Rust/Zig on allocation-heavy hot loops even with
  Tier 1 — accepted: our niche is reliable services, where predictable
  tail latency beats peak throughput.
- Tier 1's scoped arenas need a small compile-time escape check; that is
  the deliberately-bounded slice of region typing we take on instead of
  full lifetimes (budget guarded by P2/P7).

## Resolution

Adopted as **Pillar P12 (memory)**: *worry-free by default on per-process
heaps; structured, safe, colorless control on demand; `unsafe` only at the
FFI edge; control changes performance, never safety.* Feeds RQ2 (GC +
message-copy policy) and adds sample program 11 to RQ3: a hot-path buffer
pipeline written in Tier 0, then Tier 1, demonstrating the delta is local
and the function signatures don't change.
