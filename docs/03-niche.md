# 03 — Niche discovery

## Evaluation criteria

A candidate niche must be scored, not vibed. Criteria, each 1–5:

1. **Pain** — is there a real, felt, unmet need (not a theoretical one)?
2. **Durability of differentiation** — does the gap survive the
   *trajectories* of incumbents (Elixir's typing project, Go's evolution,
   TS's tooling speedup), not just their current state?
3. **Feasibility** — can a tiny team reach a credible v0.1 in ~18 months?
4. **Adoption wedge** — is there an identifiable first user with a reason
   to switch *now*, and an interop story that avoids ecosystem cold-start?
5. **Fit** — does it actually use all four influences, or are some dead
   weight?

## Candidates

### A. "Typed BEAM done right" — compete with Gleam/Elixir on the BEAM
Pain 4 · Differentiation **1** · Feasibility 4 · Wedge 2 · Fit 3 → **14/25**
Two credible incumbents are converging on this from both directions
(doc 02). Whatever we build there is a third wheel. **Rejected.**

### B. "The better backend TypeScript" — typed, batteries-included, single binary
Pain 5 · Differentiation 2 · Feasibility 3 · Wedge 3 · Fit **2** → **15/25**
The biggest market and the realest pain — but crowded (Go, Kotlin,
Bun/Deno-era TS itself), and it uses nothing of Elixir: without the
concurrency/reliability story we'd be differentiated on taste alone.
**Rejected as the niche — retained as the expansion market** (the wedge
users in C are largely these teams).

### C. "Reliable-services language" — Erlang's runtime model, TS-grade types, Go-grade logistics
Pain 4 · Differentiation 5 · Feasibility 3 · Wedge 4 · Fit 5 → **21/25**
A statically-typed, actor-based language where supervision, isolation, and
preemptive scheduling live in a runtime that compiles into **one static
binary**, with a one-binary toolchain and Python-class surface readability.
Doc 02's gap map shows this triangle is empty and the incumbents'
trajectories don't reach it: Elixir won't leave the BEAM or reach
soundness; Gleam won't leave the BEAM; Go won't adopt rich types or
supervision; Swift won't prioritize server isolation semantics.
**Selected.**

### D. "Scripting that hardens" — gradual from dynamic to typed
Pain 3 · Differentiation 2 · Feasibility 2 · Wedge 2 · Fit 2 → **11/25**
Two decades of gradual-typing research and every retrofit in doc 01 §5 show
the road from dynamic to typed is the expensive direction, and Elixir +
Python-with-pyright already own it. **Rejected.**

### E. "The language for the AI-maintenance era"
Pain 3 (rising) · Differentiation 4 · Feasibility 4 · Wedge **2** · Fit 4 → **17/25**
Real and unoccupied (doc 02), but weak as a *standalone* identity: in 2026
nobody adopts a language because models like it — they point models at Go.
And its concrete design implications (canonical style, sound contracts,
annotated boundaries, small orthogonal core, machine-readable diagnostics)
are things C wants anyway. **Adopted as a first-class design lens and
positioning angle inside C, not as the niche.**

## The niche, stated precisely

> **A statically-typed, actor-based programming language for long-lived,
> concurrent services — Erlang's fault-tolerance model with a sound,
> TypeScript-class type system, Python-class readability, and Go-class
> tooling, compiling to a single static binary. Designed from day one for
> codebases that are largely machine-written and human-maintained.**

Influence audit: Elixir supplies the runtime semantics, TypeScript the
type-system ergonomics, Go the toolchain/deployment/compile-speed doctrine,
Python the surface readability. Nothing is dead weight; each influence also
has an explicit "reject" list (doc 01) that keeps the design from becoming
a kitchen sink.

### Target user (the wedge)

Teams of ~5–50 running **fleets of long-lived backend services** who are
currently on backend TypeScript or Python and hitting the wall: refactors
are frightening, concurrency is bolted on, deployment drags a runtime +
dependency tree, incidents come from unhandled partial failure. They find
Go too blunt (no sum types, error ceremony, DIY reliability) and Rust too
expensive (learning curve, compile times, async complexity). Secondary
wedge: Elixir teams who want types and single-binary ops without leaving
the actor mental model.

### Anti-personas (explicit non-targets for v1)

- Systems/embedded programming (Zig/Rust own it; we require a runtime).
- AI kernels / numerics (Mojo's niche).
- Browser front-end (maybe a far-future compile target; not v1).
- Scripting/glue and data science one-offs (Python keeps them).

## Riskiest assumptions — ranked, with validation plans

1. **Typed actors can be made *ergonomic*, not merely possible.**
   The moat and the biggest unknown. Gleam's typed-OTP subset, Akka Typed's
   awkwardness, and Pony's learning-curve failure are all warnings.
   *Validate first and cheapest:* paper-design the actor/message/supervision
   API and write the 10 sample programs (doc 05, RQ1 + RQ3) before any
   implementation. Kill criterion: if supervised typed actors need more
   ceremony than the equivalent GenServer, the design isn't done.
2. **A small team can build a preemptive, isolated, native runtime.**
   Pony and Go prove the pieces individually; nobody has shipped all of
   per-process heaps + preemption + supervision + single binary from a
   small team. *Validate:* Phase-0 feasibility study (doc 05, RQ2) with a
   scope-cutting ladder (e.g. cooperative-with-safepoints before fully
   preemptive; shared immutable heap + per-process nurseries before full
   isolation).
3. **Teams will adopt a new language without a mature package ecosystem.**
   The #1 historical killer. *Mitigate by design:* fat standard library
   (Go's lesson), plus a serious C-ABI FFI, plus one strategic interop
   bet (doc 05, RQ4 — candidates: BEAM-target escape hatch, WASM
   components, or embedding-first à la Roc platforms).
4. **The AI-era tailwind is real and durable.** Low cost if wrong — every
   property it demands is independently justified. Track, don't bet.

## Positioning one-liners (working, name TBD)

- "Erlang's reliability. TypeScript's types. Go's toolchain. Python's
  readability. One binary."
- "The language for services that must not die — and codebases that must
  not rot."
- For the AI lens: "A language whose guarantees make machine-written code
  safe to merge."
