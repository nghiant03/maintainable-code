# Design principles

Apply these rules to implementations, callers, tests, and documented boundaries. Flag an observable maintenance or behavior consequence, not a stylistic difference. Examples are illustrative, not inferred project facts.

| Rule | Inspect and flag when | Correction / example | Keep when |
|---|---|---|---|
| D1 — Coherent responsibilities | Trace callers and side effects. Flag unrelated responsibilities when using one forces an unwanted effect or changes for a different business reason. | Separate invoice preview calculation from a retrying network write; preserve rounding and verify preview does not persist. | The steps form one operation with an explicit combined contract. Length alone is not a finding. |
| D2 — Information hiding | Trace what callers must know about storage, transport, or internal sequencing. Flag leaked details that callers repeatedly reconstruct or must change together. | Move repeated persistence sequencing behind an operation that owns it. | Low-level control is intentionally part of the interface contract. |
| D3 — Useful abstractions | Follow a representative call through wrappers, interfaces, and factories. Flag indirection that adds navigation without hiding details, simplifying callers, or protecting an intentional boundary. | Inline a forwarding-only helper with no separate contract instead of adding another layer. | A one-implementation interface preserves a documented boundary or supplies a meaningful test seam. |
| D4 — Single source of business knowledge | Compare duplicated calculations and decisions and identify why they would change. Flag independent copies of the same business rule. | Centralize the shared eligibility policy and test its consumers. | Similar code serves different policies that can evolve independently. Textual similarity alone does not justify merging. |
| D5 — Behavior-preserving changes | Compare the proposed change with public fields, serialization, errors, ordering, and side effects. Flag an unrequested contract change or a refactor without relevant verification. | Separate a rename from a behavior change; add a boundary regression test and use the agreed migration for public fields. | A behavior change is explicitly requested and its compatibility plan is agreed. |
| D6 — Agreed architecture | Read the documented dependency direction, then inspect the resolved dependency graph. Flag a forbidden edge or cycle covered by project policy. | Remove the dependency through the existing boundary; add allowed, forbidden, and exception fixtures to its check. | No agreed boundary forbids the edge. Report the unknown rather than inventing a layering scheme. |
| D7 — Failure and retry contracts | Trace timeout, partial-success, retry, and duplicate-delivery paths in data/distributed changes. Flag a supported failure scenario with unspecified recovery or inconsistent effects. | Reuse a stable request ID across retries when the receiver's deduplication contract supports it; test a timeout after acceptance. | No such failure path applies, or the existing contract and tests already cover it. Do not assume exactly-once delivery. |
| D8 — Schema evolution | Compare old/new readers and writers, serialized fields, and rollout order. Flag a schema change that breaks supported consumers or loses data without an agreed migration. | Use a compatible rollout and test supported old/new reader-writer combinations before removing a field. | No old data or consumers survive the change, or an explicitly agreed breaking migration covers both. |

## Resolve tensions in favor of reader cost

Very small functions can help when they name a coherent operation. They can hurt when understanding one operation requires jumping through several one-line wrappers. Judge the amount of context the next maintainer needs, not line counts.

An abstraction earns its place by simplifying callers, hiding unstable details, or representing shared domain knowledge. Avoid introducing interfaces, factories, or layers for hypothetical variation. Conversely, removing an interface solely because it currently has one implementation can discard an intentional boundary.

Use [Commenting Principles](commenting-principles.md) to document contracts, constraints, non-obvious algorithms, and rationale. Comments can explain both why a choice exists and what an interface promises; remove redundant narration only when the code conveys the same information. Apply language-appropriate error handling rather than universal rules borrowed from another ecosystem.

## Make architecture executable where justified

- Start from documented boundaries and real maintenance failures.
- Use the repository's dependency tooling for graph-wide restrictions and cycles. A changed import can create a cycle through unchanged files.
- Keep simple scripts simple; they need not adopt enterprise layers.
- Add positive and negative fixtures for custom rules, including legitimate exceptions.
- Introduce only a few high-value rules. Keep complexity, length, and duplication warnings advisory until evidence supports enforcement.
- Existing debt may need explicit exceptions in the repository's own tooling. Do not regenerate a baseline after each failing change.

## Evidence for a useful finding

Weak: “This method is long; split it.”

Useful: “`billing.py:82` mixes tax calculation with a retrying network write. Callers calculating a preview can create an invoice. Separate calculation from persistence, preserve rounding behavior, and add a regression check at the preview boundary.”

State the observable consequence and propose the smallest remedy that addresses it. Check behavioral contracts with tests that exercise caller-visible outcomes rather than mirror implementation steps.

## Source pointers

- [Clean Code](https://www.informit.com/store/clean-code-a-handbook-of-agile-software-craftsmanship-9780132350884)
- [Ousterhout's official second-edition extract](https://web.stanford.edu/~ouster/cgi-bin/aposd2ndEdExtract.pdf)
- [Refactoring](https://martinfowler.com/books/refactoring.html)
- [The Pragmatic Programmer](https://pragprog.com/titles/tpp20/the-pragmatic-programmer-20th-anniversary-edition/)
- [Architectural fitness functions](https://www.thoughtworks.com/en-us/insights/articles/fitness-function-driven-development)
- [Designing Data-Intensive Applications](https://dataintensive.net/)

These are a synthesis of the supplied research transcript, not claims that this bundle mechanically enforces an entire book.
