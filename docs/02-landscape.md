# 02 — Landscape survey (as of mid-2026)

A niche is only real if nobody credible occupies it *and* the incumbents'
trajectories don't reach it soon. This survey covers both. Statuses below
reflect mid-2026.

## Incumbents converging on "our" territory

### Elixir — the incumbent absorbing the critique
Elixir v1.20 (June 2026) is now a gradually typed language: set-theoretic
types with inference over all constructs, no annotations required, dead-code
and guaranteed-runtime-failure reporting. Type signatures and typed structs
are explicitly on the roadmap for the next ~15 months.
**Take-away:** "Elixir but typed" is not an open niche — Elixir itself is
filling it. But the retrofit is bounded: gradual (not sound), constrained by
existing idioms and the BEAM, messages remain dynamically shaped, and the
deployment/CPU-performance story is unchanged.
Sources: [Elixir v1.20 release](https://elixir-lang.org/blog/2026/06/03/elixir-v1-20-0-released/),
[type inference of all constructs and the next 15 months](https://elixir-lang.org/blog/2026/01/09/type-inference-of-all-and-next-15/),
[gradual set-theoretic types docs](https://hexdocs.pm/elixir/gradual-set-theoretic-types.html).

### Gleam — the other typed-BEAM occupant
v1.16 (April 2026), ~21k GitHub stars, 70% admiration in the 2025 Stack
Overflow survey (second only to Rust), Thoughtworks Radar "Assess" ring.
Simple sound HM-style types on the BEAM (and JS), interop with
Elixir/Erlang, most users arriving from *outside* the BEAM world.
**Positioning:** deliberately minimal — a small expression language; no
macros, restrained abstraction; typed-OTP via libraries covering a subset
of OTP. Tied to the BEAM's deployment and performance profile.
**Take-away:** the "simple sound types on BEAM" corner is taken, and taken
well. Competing there head-on fails the differentiation test.
Sources: [Gleam overview 2026](https://www.programming-helper.com/tech/gleam-programming-language-2026-type-safe-erlang-beam),
[Gleam Gathering 2026 reflections](https://alembic.com.au/blog/gleam-gathering-2026),
[Wikipedia](https://en.wikipedia.org/wiki/Gleam_(programming_language)).

### Go — the tooling/deployment incumbent
Owns cloud infrastructure and the single-binary deployment story; generics
(since 1.18) slowly maturing; still no sum types, still `nil`, error
ceremony unchanged. Notably, Go is emerging as a favored target for
LLM-generated code because of its uniformity and explicitness.
**Take-away:** Go's *trajectory* does not lead toward a rich type system or
supervision-style concurrency — the team's philosophy forbids it. The gaps
we'd exploit are stable, durable gaps.
Sources: [Why Go for LLM code generation](https://medium.programmerscareer.com/why-go-is-the-best-programming-language-for-llm-code-generation-741a90ff7201),
[Applied Go: best language for AI-assisted coding](https://appliedgo.net/spotlight/the-best-language-for-ai-assisted-coding/).

### TypeScript itself (Node/Bun/Deno backends)
The default backend choice for TS-native teams. Unsound types, no real
parallelism story (workers + message passing bolted on), heavy runtime
dependency chains, `node_modules` deployment. The 10× faster Go-based TS
compiler (tsc port) improves tooling speed but changes none of the
semantics. **Take-away:** the pain we target is felt *most acutely* by
exactly these teams — they are the adoption wedge, not the competition.

## Mainstream typed alternatives (why they don't close the gap)

| Language | Has | Lacks for our niche |
|---|---|---|
| Kotlin | Ergonomic types, coroutines | JVM ops weight; structured concurrency is library-level; no supervision model; slow cold builds |
| Swift | Sound ergonomic types, actors (!) | Server story perpetually secondary; Apple-gravity; actor model is shared-memory-adjacent, no supervision trees |
| C# | Mature types, async | Ecosystem gravity toward Windows/enterprise; no isolation model; large runtime |
| Rust | Soundness, performance, sum types | Steep curve fights "Python readability" head-on; async ecosystem complexity; LLMs demonstrably struggle with lifetimes and reach for `unsafe`/`unwrap` |
| Java (Loom) | Virtual threads at scale | Shared-memory concurrency; ceremony; no supervision; ops weight |

Swift deserves note: it is the closest *type-system* relative of what we
want (sound, ergonomic, value-oriented, actors). Its gap is the runtime
model (no isolation/supervision/preemption guarantees) and its center of
gravity. Worth deep study in Phase 0 rather than dismissal.

## Newer languages and cautionary tales

- **Crystal** ("typed Ruby") — the central cautionary tale for us: global
  whole-program inference produced beautiful code and compile times that
  collapsed at scale, plus a decade of ecosystem cold-start. Lessons:
  module-local inference, compile-speed budget from day one, interop plan
  from day one.
- **Mojo** — Python-superset for AI kernels/GPUs; jumped TIOBE #194 → #68
  in a year. Owns the AI-performance niche; zero overlap with reliable
  long-lived services. Its lesson is strategic: *pick a sharp niche and an
  interop host* ([Semaphore survey](https://semaphore.io/blog/programming-languages-2025)).
- **Zig** — C-successor systems niche; growing among infrastructure
  engineers. Different layer of the stack; a candidate *implementation
  language* for our runtime rather than a competitor.
- **Roc** — FP with strong safety + performance and friendly DX; early,
  platform-based architecture is interesting prior art for embedding a
  runtime. Watch, study `roc` platforms.
- **Nim, D** — power without positioning discipline; both show that
  feature breadth without a sharp niche stalls adoption.
- **Pony** — commercially dead but technically essential prior art: proves
  actors + per-actor heaps + a *type system that makes data races
  impossible* (reference capabilities) can run native and fast. Its
  failure was ergonomics/learning curve, not the runtime model.
- **Unison / Darklang / Ballerina** — distributed-first experiments with
  little adoption; idea mines (content-addressed code, infra-in-language)
  but evidence that "distributed-first" alone doesn't sell.

## The meta-trend: specialization + host-ecosystem interop

Across the 2025–26 surveys the pattern is consistent: new languages that
grow share a sharp niche and a parasitic interop story — Mojo→Python,
Gleam→BEAM/Hex, Zig→C, TypeScript→JS. Empty-ecosystem cold start is the
#1 killer of well-designed languages
([Semaphore](https://semaphore.io/blog/programming-languages-2025),
[Bits Kingdom overview](https://bitskingdom.com/blog/programming-languages-open-source-2026/)).
**Implication:** our interop/host decision (doc 05, RQ4) is as important as
the type system.

## The AI-era lens (unoccupied, rising)

By 2026 a large share of new code in industry is LLM-written and
human-reviewed. Discussion of "a language designed for LLMs" is active but
no credible language occupies it; today teams point LLMs at Go for its
uniformity and explicitness, and note LLMs fail worst where languages are
cleverest (Rust lifetimes, TS type golf)
([ploeh: programming languages for AI](https://blog.ploeh.dk/2026/03/30/programming-languages-for-ai/),
[Applied Go](https://appliedgo.net/spotlight/the-best-language-for-ai-assisted-coding/)).
Properties that serve AI-written/human-reviewed code — one canonical style,
annotated public boundaries, sound checked contracts, small orthogonal
feature set, precise machine-readable diagnostics — are *the same
properties* our influences already demand. This is a free tailwind: design
for it deliberately, market it honestly, but don't depend on it as the
sole identity.

## Gap map

Plotting the field on the two axes that define our thesis:

```
                     weak/no fault-tolerance model        supervision/isolation-native
                    ┌──────────────────────────────────┬──────────────────────────────┐
  sound/rich types  │ Rust, Swift, Kotlin, Haskell     │ Gleam (BEAM-bound, minimal)  │
                    │ (no supervision, shared memory)  │ Pony (dead, ergonomics)      │
                    │                                  │        ← THE GAP →           │
  weaker/unsound or │ TypeScript, Go, Python           │ Elixir/Erlang (gradual types,│
  gradual types     │                                  │ BEAM-bound)                  │
                    └──────────────────────────────────┴──────────────────────────────┘
                                 + third axis: Go-class tooling & single-binary deploys —
                                   possessed by NO ONE in the right-hand column.
```

Nobody offers: **sound ergonomic types + isolation/supervision runtime +
one-binary toolchain and deployment**. That triangle is the niche examined
in doc 03.
