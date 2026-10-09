# Design principles

Use these as questions grounded in the code.

| Source | Useful question |
|---|---|
| Robert C. Martin, *Clean Code*, chapters 2–3 | Do names reveal intent, responsibilities cohere, and side effects match the contract? |
| John Ousterhout, *A Philosophy of Software Design* | Does the interface hide substantial complexity, or merely move it into more files and calls? |
| Martin Fowler, *Refactoring*, second edition | Can this change be made as small behavior-preserving steps with relevant verification? |
| David Thomas and Andrew Hunt, *The Pragmatic Programmer* | Is business knowledge duplicated, or is this only incidental textual similarity? |
| Neal Ford, Rebecca Parsons, Patrick Kua, and Pramod Sadalage, *Building Evolutionary Architectures*, second edition | Which agreed architectural property can an executable check protect? |
| Martin Kleppmann, *Designing Data-Intensive Applications* | For data/distributed changes, are failure behavior, consistency, retries, idempotency, and schema evolution explicit? |

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

Prefer an observable consequence and the smallest remedy. Check behavioral contracts with meaningful tests rather than tests that mirror the implementation.

## Source pointers

- [Clean Code](https://www.informit.com/store/clean-code-a-handbook-of-agile-software-craftsmanship-9780132350884)
- [Ousterhout's official second-edition extract](https://web.stanford.edu/~ouster/cgi-bin/aposd2ndEdExtract.pdf)
- [Refactoring](https://martinfowler.com/books/refactoring.html)
- [The Pragmatic Programmer](https://pragprog.com/titles/tpp20/the-pragmatic-programmer-20th-anniversary-edition/)
- [Architectural fitness functions](https://www.thoughtworks.com/en-us/insights/articles/fitness-function-driven-development)
- [Designing Data-Intensive Applications](https://dataintensive.net/)

These are a synthesis of the supplied research transcript, not claims that this bundle mechanically enforces an entire book.
