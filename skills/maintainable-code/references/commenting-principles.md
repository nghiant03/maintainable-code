# Commenting principles

Write comments that save readers from reconstructing a contract or design decision. Judge usefulness from the perspective of someone encountering the code for the first time. These rules apply across languages; use the repository's doc-comment syntax and conventions.

## When to comment

| Situation | Useful action |
|---|---|
| A caller cannot learn the contract from the declaration | Document behavior, input constraints, results, and relevant side effects or failures at the interface. |
| A value has meaning its type or name does not express | Explain units, valid ranges, inclusive/exclusive bounds, sentinel values, ownership, or lifetime where declared. |
| An implementation choice looks arbitrary or invites an incorrect simplification | Explain the constraint, rationale, or failure that the choice prevents beside the affected code. |
| Several steps implement a non-obvious algorithm or phase | Give a short high-level explanation of the goal or invariant before the block. Avoid translating each statement into prose. |
| A rule spans modules | Keep one authoritative explanation and reference it from the affected locations. Prefer existing design notes over duplicating the rule. |
| A comment merely repeats the name, type, or next statement | Omit it, or remove it after checking that no useful information is lost. |
| Prose compensates for a misleading name or unnecessarily tangled logic | Consider a clearer name or a small refactor first. Keep any contract or rationale the improved code still cannot convey. |

“Comment why, not what” is too narrow. An interface needs to describe what it promises; a block comment can explain an algorithm's overall purpose. The waste is repeating code at the same level of detail.

## How to write the comment

### Interface comments: help callers use the code

Start with the behavior or abstraction. Add the details needed to use it correctly without opening the implementation: parameter meaning, preconditions, return semantics, boundary cases, mutation, I/O, failure behavior, and ownership or concurrency obligations when relevant. Document applicable details, not empty template sections.

Keep internal mechanics out unless callers depend on them as part of the contract. A signature can show an integer parameter without telling the reader whether it counts bytes, accepts zero, or includes the end position.

### Implementation comments: help maintainers change the code

Place the explanation at the narrowest relevant scope. State the reason for a surprising choice, the invariant a block preserves, or the dependency a maintainer must keep in mind. Use precise conditions instead of vague labels such as “handle edge cases” or “performance optimization.”

The following examples are illustrative, not inferred project facts:

```text
Redundant: Increment the retry count.
Useful:   Reuse the request ID on retries so the receiver can reject duplicates.

Vague:    Validate the range.
Contract: start is inclusive; end is exclusive. Equal bounds return an empty result.

Vague:    Do not move this call.
Useful:   Publish only after commit; subscribers may read the record immediately.
```

Include a short local explanation even when linking to an issue or design note. The reference supplies detail; the comment should still identify the relevant constraint.

## Write and maintain comments with the code

For a new or substantially changed interface, draft its contract before implementing the body. If the description is difficult to make clear, reconsider the responsibilities or interface. Revise the draft as the design develops.

When changing behavior, review nearby comments and the authoritative contract in the same change. If code and prose disagree, investigate the intended behavior before changing either. Preserve useful rationale through refactoring and move it with the code it explains. Prefer references over copied explanations that can drift apart.

For agent-authored comments, verify claims against code, tests, specifications, or recorded decisions. Do not invent historical reasons, issue references, performance guarantees, or thread-safety promises. Report an unknown when the evidence does not establish it. Preserve required notices and tool directives; they may have purposes beyond explaining the code.

## Review questions

- What would the intended reader otherwise have to infer or look up?
- Is the statement precise, supported, and consistent with current behavior?
- Is it located where the reader needs it, with one authoritative version?
- Would better code convey the same information more clearly, without creating needless indirection?

Comment counts, line ratios, and mandatory prose on every trivial operation do not answer these questions. Review meaning in context.

## Sources and interpretation

Revisited on 2026-10-08 using these author or publisher sources:

- [John Ousterhout, CS 190: Writing Comments (2016)](https://web.stanford.edu/~ouster/cgi-bin/cs190-spring16/lecture.php?topic=comments): interface versus implementation documentation, precise value semantics, placement, avoiding duplication, and writing comments during design.
- [Ousterhout, A Philosophy of Software Design, second-edition extract](https://web.stanford.edu/~ouster/cgi-bin/aposd2ndEdExtract.pdf), section 12.6, printed pages 99–100: explicitly disagrees with treating comments as design failures; interface descriptions reduce the need to inspect implementations. The extract contains selected pages, not the complete commenting chapters.
- [Robert C. Martin, Avoid Redundant Comments](https://www.informit.com/articles/article.aspx?p=1327761): removes commentary that repeats adequately expressed code while retaining a concise behavior description. The publisher's [Clean Code contents](https://www.informit.com/store/clean-code-a-handbook-of-agile-software-craftsmanship-9780132350884) locate the broader discussion in chapter 4; the full chapter was not reviewed here.
- [Martin Fowler, Extract Function](https://refactoring.com/catalog/extractFunction.html): demonstrates replacing a block-label comment with a named operation. Apply when extraction improves understanding, not as a rule to eliminate comments.
- [The Pragmatic Programmer, publisher's tips](https://pragprog.com/tips/), tips 13, 15, and 37: maintain documentation with code, avoid duplicating knowledge, and make contracts explicit.

The decision table and agent-specific safeguards are this skill's synthesis. Ousterhout's course recommends broad declaration documentation; Martin emphasizes removing redundancy. This skill requires useful contract coverage and follows repository conventions rather than imposing a universal comment quota.
