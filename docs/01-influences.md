# 01 — Analysis of the influences

The starting intuition names four influences and one observation. This
document decomposes each influence into *what actually made it work*, *what
to reject*, and then confronts the tensions between them — because the four
influences are not automatically compatible, and the places where they
conflict are exactly where this language's identity will be decided.

## 1. TypeScript's type system

### What actually made it win

- **Structural typing.** Types describe the *shape* of data, not its
  declared lineage. This matched how JavaScript programmers already thought
  and meant existing code was typeable without rewriting it.
- **Union types + flow-sensitive narrowing.** `string | null`, discriminated
  unions, and `if (x.kind === "a")` narrowing model how real-world data
  actually looks (JSON, API responses, state machines) far better than
  classical class hierarchies do.
- **Inference that carries most of the weight.** Annotations cluster at
  function boundaries; locals are almost never annotated. The code stays
  readable while remaining fully checked.
- **Types as a tooling substrate, not just a correctness gate.** The type
  system's killer app is the language server: completion, rename,
  find-references, safe refactors. TypeScript is a UX product that happens
  to be a type system.
- **A migration story.** `any`, `// @ts-ignore`, and per-file adoption meant
  teams could adopt it incrementally. (Note: this mattered because TS was
  retrofitting JS. A greenfield language doesn't need the escape hatches —
  but it *does* need the lesson that adoption paths matter; see doc 03.)

### What to reject

- **Unsoundness as a permanent tax.** `any`, type assertions, bivariant
  method params, and erasure mean TS types are advisory. Whole classes of
  "the type said X but runtime had Y" bugs survive. A greenfield language
  is not bound by JS compatibility and should have a **sound core**.
- **A Turing-complete type level.** Conditional types, mapped types,
  template-literal types produced a "type golf" subculture, inscrutable
  errors, and compile-time blowups. Expressivity needs a budget.
- **Configuration sprawl.** Dozens of `tsconfig` strictness flags mean
  "TypeScript" names a family of dialects. One language, one meaning.
- **Types with no runtime existence.** Erasure forces the ecosystem to
  re-validate at every boundary (zod et al. exist because types vanish).
  Types should be checkable at runtime boundaries without a parallel
  schema language.

## 2. Go's tooling philosophy

### What actually made it work

- **The toolchain is one binary and part of the language.** `go build`,
  `go test`, `go fmt`, `go vet`, `go mod` — no plugin ecosystem to
  assemble, no build-tool wars. A new team member is productive in
  minutes.
- **`gofmt` has no options.** Ending format debates by fiat turned out to
  be one of the highest-leverage decisions in modern language design;
  every serious language since has copied it.
- **Compile speed as a hard design constraint.** Go's dependency and
  export rules were *designed* so builds stay fast. Fast builds are why
  the edit-test loop feels dynamic even though the language is static.
- **Single static binary deployment.** `scp` the binary; done. This is a
  language feature, not an ecosystem accident, and it is a large part of
  why Go owns cloud infrastructure.
- **The compatibility promise.** Code from 2012 still builds. Boring is a
  feature in a language for long-lived codebases.

### What to reject

- **Type-system austerity as ideology.** Generics arrived 12 years late;
  `nil` still ships; sum types and non-nullability are still missing.
  Austerity pushed real-world complexity into `interface{}`, codegen, and
  runtime panics. Simplicity should cap *cleverness*, not *safety*.
- **`if err != nil` as one third of every function.** Errors-as-values is
  right; the ceremony is not. (See doc 04: result types with propagation
  sugar.)
- **Concurrency without supervision.** Goroutines are cheap but unmanaged:
  no ownership, no supervision, leaked goroutines, shared-memory races
  guarded only by convention and `-race`. Elixir's model is stronger here.

## 3. Python's readability

### What actually made it work

- **Syntax optimized for reading, not writing.** Code is read far more
  often than written; indentation blocks, minimal sigils, and
  keyword-over-symbol choices make Python read like pseudo-code. This is
  why it conquered teaching, science, and now AI.
- **Low concept count for the common path.** You can hold beginner Python
  in your head. Features arrive progressively.
- **"One obvious way to do it."** A cultural norm that reduces dialect
  formation inside teams — same goal as `gofmt`, achieved socially.

### What to reject

- **Readability via dynamism.** Python's clean look historically leaned on
  the absence of types and on runtime flexibility (monkey patching,
  metaclasses) — precisely the things that hurt at scale. The lesson is
  that readability must come from *syntax design + inference*, not from
  omitting information the maintainer needs.
- **The retrofit itself as an endorsement of the critique.** Type hints,
  mypy/pyright as de-facto industry standard, dataclasses, protocols —
  Python has spent 15 years bolting on what a scale-ready language needs
  built in. The bolt-on remains optional, partial, and unsound.
- **Packaging/deployment sprawl** (venvs, wheels, lockfile wars) — the
  anti-Go. Deployment pain is a language-adoption issue, not a detail.

## 4. Elixir's concurrency ideas

### What actually made it work (mostly BEAM/OTP inheritance)

- **Processes as the unit of both concurrency and isolation.** Millions of
  lightweight processes, each with its own heap, communicating only by
  message passing. No shared mutable memory → no data races by
  construction, and per-process GC → no global pauses.
- **Preemptive scheduling.** A busy-looping process cannot starve the
  system. This is a *reliability* property, not just a performance one.
- **Supervision trees and "let it crash."** Failure handling is
  architectural: you declare restart strategies instead of writing
  defensive try/catch everywhere. This is the single most under-copied
  good idea in industrial language design.
- **OTP as codified patterns.** GenServer et al. show the value of
  shipping *behavioral* building blocks in the standard library, not just
  data structures.
- **Immutability by default**, which is what makes the above tractable.

### What to reject

- **Dynamic typing** — and this is now conceded by the incumbent: Elixir
  v1.20 (June 2026) ships gradual set-theoretic types, with inference over
  all constructs and type signatures still on the roadmap. The demand is
  proven; the retrofit is constrained by gradualness and existing idioms.
  A greenfield language can have the runtime model *and* sound types.
- **BEAM as the only vessel.** The BEAM brings a CPU-performance ceiling,
  a heavyweight ops/deployment story (releases, epmd, its own tooling
  culture), and NIF danger zones. The *semantics* (isolation, preemption,
  supervision) are separable from the *VM*; Pony and structured-
  concurrency research show pieces of this on native runtimes.
- **Untyped messages as the price of actors.** `handle_call` on arbitrary
  terms is where Elixir's dynamism bites hardest. Typing the messages —
  not just the functions — is the central technical research problem for
  this language (see doc 05, RQ1).

## 5. The observation: dynamic languages don't scale — evidence

This is no longer a matter of taste; every major dynamic ecosystem has
independently converged on the same correction:

| Ecosystem | Retrofit | Status |
|---|---|---|
| JavaScript | TypeScript | De-facto standard for new codebases |
| Python | Type hints + mypy/pyright | Industry-standard in serious teams |
| Ruby | Sorbet / RBS | Adopted at Stripe, Shopify |
| PHP | Gradual type declarations | Mainstream since PHP 7 |
| Elixir | Set-theoretic gradual types | Shipped v1.20, 2026 |

The mechanism is well understood: beyond roughly one team-boundary of code,
maintenance is dominated by *reading, navigation, and safe change* — and
those are exactly what checked interfaces buy. Conclusion for us: **start
statically typed with dynamic-feeling ergonomics** (inference, structural
types, REPL-grade feedback loops). Do not start dynamic and harden later;
every language above shows how expensive that road is.

## 6. Tensions between the influences

These four influences are not free to combine. Naming the conflicts now
prevents incoherent design later:

1. **TS expressivity vs Go simplicity.** A type system rich enough for
   unions/generics/narrowing, but with a hard budget: no type-level
   programming. The line must be drawn explicitly and defended in the spec.
2. **Python readability vs static annotation noise.** Resolution: infer
   everything *inside* function bodies; require annotations only at public
   boundaries (which doubles as documentation and API stability — and, per
   doc 03, as anchors for AI-generated code).
3. **Powerful global inference vs Go-class compile speed.** Crystal is the
   cautionary tale: whole-program inference gave it Ruby's look and
   unusably slow builds at scale. Resolution: module-local inference with
   explicit module boundaries — compile speed is a hard budget (doc 04).
4. **Actor isolation vs static typing.** Typed messages, typed process
   references, typed supervision — genuinely hard, only partially solved
   anywhere (Gleam's typed-OTP subset, Akka Typed's awkwardness, Pony's
   reference capabilities). This is our primary research bet: if it works,
   it is the moat; if it can't be made ergonomic, the design must change.
5. **Erlang-style preemptive runtime vs single-static-binary deployment.**
   We want BEAM semantics with Go logistics: a runtime with preemptive
   green processes compiled *into* one native binary. Feasible (Go ships a
   preemptive scheduler in-binary) but the isolation + per-process-heap
   design is real engineering (doc 05, RQ2).
