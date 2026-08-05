# lang-design

Rigorous planning and research for a new programming language.

## The thesis in one paragraph

Every major dynamic language of the last 30 years has spent the last decade
retrofitting static types — JavaScript grew TypeScript, Python grew type
hints and pyright, Ruby grew Sorbet, and Elixir shipped a gradual
set-theoretic type system in v1.20 (2026). The market has spoken: dynamic
languages do not thrive in large, long-lived codebases. Meanwhile, no
language combines the four things that demonstrably work at scale —
**TypeScript's developer-facing type system**, **Go's tooling philosophy and
deployment story**, **Python's readability**, and **Elixir's
concurrency-and-fault-tolerance runtime model**. The research in this repo
decomposes those influences, surveys the 2026 landscape, and identifies the
defensible niche: a **statically-typed, actor-based language for reliable
long-lived services, compiled to a single static binary** — with "code that
is increasingly written by AI and reviewed by humans" as a first-class
design lens.

## Documents

| Doc | Contents |
|---|---|
| [01 — Influences](docs/01-influences.md) | What each of the four influences *actually* contributes, what to reject from each, and the tensions between them |
| [02 — Landscape](docs/02-landscape.md) | Prior-art survey with 2026 status: who occupies which corner, and the gap map |
| [03 — Niche](docs/03-niche.md) | Candidate niches evaluated against explicit criteria; positioning; riskiest assumptions |
| [04 — Design principles](docs/04-design-principles.md) | Pillars, hard budgets, and non-goals derived from 01–03 |
| [05 — Roadmap](docs/05-roadmap.md) | Open research questions, phased plan, and kill criteria |

## Status

**Phase 0 — niche discovery and research.** No code, no syntax commitments
yet. Per the roadmap, the first design artifacts will be *sample programs*,
not a compiler.

## Working conventions

- Every significant decision gets a short ADR (architecture decision record)
  in `docs/decisions/` once we start making them.
- Claims about other languages cite sources; claims about our own design
  cite a sample program or a spike.
