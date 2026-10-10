# Commenting principles

Write comments that save readers from reconstructing a contract or design decision. Judge usefulness from the perspective of someone encountering the code for the first time. These rules apply across languages; use the repository's doc-comment syntax and conventions.

## When to comment

| Situation | Useful action |
|---|---|
| A caller cannot learn the contract from the declaration | Document behavior, input constraints, results, and relevant side effects or failures at the interface. |
| A value has meaning its type or name does not express | Explain units, valid ranges, inclusive/exclusive bounds, sentinel values, ownership, or lifetime where declared. |
| An implementation choice looks arbitrary or invites an incorrect simplification | Explain the constraint, rationale, or failure that the choice prevents beside the affected code. |
| Several steps implement a non-obvious algorithm or phase | Give a short high-level explanation of the goal or invariant before the block. Avoid translating each statement into prose. |
| A rule spans modules | Keep one authoritative explanation and reference it from the affected locations. Use an existing design note if it documents the rule; otherwise document the rule once at its owning interface or in a design note. |
| A comment merely repeats the name, type, or next statement | Omit it, or remove it after checking that no useful information is lost. |
| Prose compensates for a misleading name or unnecessarily tangled logic | Consider a clearer name or a small refactor first. Keep any contract or rationale the improved code still cannot convey. |

“Comment why, not what” is too narrow. An interface needs to describe what it promises; a block comment can explain an algorithm's overall purpose. The waste is repeating code at the same level of detail.

## How to write the comment

### Interface comments: help callers use the code

Start with a short statement of the behavior or abstraction. Add only details callers need to use it correctly without opening the implementation. These may include parameter meaning, preconditions, return semantics, boundary cases, mutation, I/O, failures, and ownership or concurrency obligations. This is a relevance checklist, not a requirement to expand every item into prose. Do not repeat information already clear from the declaration or fill empty template sections.

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

### Linear, economical prose

Comments compete with code for the reader's attention. State the main behavior or reason directly, with one coherent claim per sentence. Use concrete subjects and verbs rather than an introduction such as “Note that,” “It is important to remember,” or “This function is responsible for.”

- Avoid parenthetical prose by default. Integrate essential information into the sentence, give it a separate sentence, or delete an unnecessary aside. Do not merely replace parentheses with dashes, commas, or semicolons; remove the interruption itself.
- Preserve parentheses needed for code examples, formulas, citations, and tool syntax. A short parenthetical term can be useful when it genuinely clarifies the text. Do not impose a punctuation ban or alter executable code to satisfy this prose guidance.
- Split independent claims into separate sentences. Keep a semicolon only when it clarifies a close relationship between the claims. Do not chain conditions, caveats, or unrelated instructions with `;`. Preserve semicolons in code examples, literal text, and tool syntax.
- Use colons to introduce a list, example, or explanation directly tied to the preceding phrase. Avoid unnecessary labels such as “Note:” or “Explanation:” when a direct sentence is clearer. Preserve colons in code examples, literal text, and tool directives.
- State precise conditions instead of accumulating qualifications. Keep a caveat when it changes how the reader must call or modify the code, not merely to make a sentence sound cautious or comprehensive.
- Do not narrate statements, explain ordinary language idioms to experienced maintainers, or paraphrase the signature. A comment should supply information the intended reader would otherwise have to reconstruct.
- Keep local comments local. Move broad workflow explanations to authoritative documentation and retain the relevant constraint and reference beside the code. Leave obsolete milestone history and change narratives to version control unless they explain a current restriction.
- Remove repeated explanations across module headers, helpers, and callers. When an interface needs a local warning to prevent misuse, retain that warning and reference the authoritative explanation instead of copying it.

The examples below illustrate editing choices, not project facts:

```text
Before: Count distinct meal slots (breakfast, lunch, and dinner), not dishes
        (multiple dishes in the same slot still count as one).
After:  Count each meal slot once, regardless of how many dishes it contains.

Before: Return a copy (the input is not mutated), or None (if no match exists).
After:  Return a copy of the match, or None if no match exists. Leave the input unchanged.

Before: It is important to note that this function performs a revision check
        (even when the optional client token is omitted) to prevent stale writes.
After:  Reject stale writes even when the client omits the revision token.

Before: Persist the record (using the database transaction), then publish it
        (after the commit has completed), so subscribers can read it safely.
After:  Publish only after commit so subscribers can read the persisted record.

Before: Return an empty list if no records match; leave the cache unchanged;
        callers must retry transient failures.
After:  Return an empty list if no records match. Leave the cache unchanged.
        Callers must retry transient failures.
```

The third example assumes the input snapshot and durable revision comparison are documented at their authoritative interface. Do not shorten a comment by erasing a distinction its reader needs.

## Write and maintain comments with the code

For a new or substantially changed interface, draft its contract before implementing the body. If the description is difficult to make clear, reconsider the responsibilities or interface. Revise the draft as the design develops.

When changing behavior, review nearby comments and the authoritative contract in the same change. If code and prose disagree, investigate the intended behavior before changing either. Preserve useful rationale through refactoring and move it with the code it explains. Reference the authoritative explanation rather than copying it; retain a short local constraint when readers need it to avoid misuse.

For agent-authored comments, verify claims against code, tests, specifications, or recorded decisions. Do not invent historical reasons, issue references, performance guarantees, or thread-safety promises. Report an unknown when the evidence does not establish it. Preserve required notices and tool directives; they may have purposes beyond explaining the code.

Review in two passes:

1. **Correctness and contract coverage:** verify each claim and identify missing information needed to use or change the code safely.
2. **Clarity and subtraction:** read the comment as prose. Remove repetition, unnecessary asides, mechanical narration, and detail that belongs elsewhere. Check the result against the first pass so brevity does not erase units, bounds, ownership, failures, invariants, or rationale.

A technically correct paragraph can still be a poor comment. Report readability problems as findings in their own right; do not limit an audit to stale or false claims.

## Concrete comment rules

Apply C1–C4 in the correctness/coverage pass, then C5–C8 to the same scope in the clarity/subtraction pass. Inspect interfaces without comments too. Examples are illustrative; verify the actual contract before proposing replacement prose.

| Rule | Inspect and flag when | Correction / example | Keep when |
|---|---|---|---|
| C1 — Supported claims | Compare each behavioral claim with implementation, tests, specifications, or recorded decisions. Flag contradictions; report unsupported guarantees as unknown rather than assuming they are false. | If a documented “no I/O” lookup writes a disk cache, resolve the intended contract and correct code or prose together. | Evidence supports the claim. Preserve notices and tool directives even when they are not explanatory prose. |
| C2 — Caller contract | Trace parameter use, outputs, failure paths, mutation, and ownership. Flag missing details that could cause a caller to use the interface incorrectly. | Document `start` inclusive, `end` exclusive, and equal bounds yielding an empty result if those facts are otherwise hidden. | The declaration, types, or authoritative interface documentation already convey the needed detail. Do not add empty template sections. |
| C3 — Rationale and invariants | Inspect surprising ordering, workarounds, and algorithm phases. Flag a missing explanation when a plausible simplification would break an established constraint. | Replace “Do not move this call” with “Publish only after commit so subscribers can read the record.” | The code makes the reason clear, or no evidence establishes a rationale. Never invent history or performance promises. |
| C4 — Location and maintenance | Compare local comments with the authoritative contract and current behavior. Flag stale explanations, detached rationale, or conflicting copies. | Move rationale with the code it explains; keep a local constraint and link to the authoritative design note. | A brief local warning prevents misuse without duplicating the full explanation. |
| C5 — Redundant narration | Compare each sentence with the adjacent name, type, and statement. Flag prose that adds no contract, rationale, or useful phase explanation. | Delete “Increment the retry count” above an obvious increment. | A block summary explains a non-obvious algorithm at a higher level than individual statements. |
| C6 — Interrupted or padded prose | Read each sentence in order. Flag introductions or asides that delay the main condition or force the reader to resume a suspended thought. | Replace “It is important to note that … (even when the token is omitted)” with “Reject stale writes even when the client omits the token.” | Parentheses convey a needed term, formula, citation, or code syntax. Punctuation alone is not a finding. |
| C7 — Vague or overloaded claims | Identify the concrete condition and consequence in each sentence. Flag labels without that information or chains of unrelated obligations. | Replace “Handle edge cases” with the actual boundary behavior. Split “Return no matches; leave the cache unchanged; retry failures” into separate claims. | A colon introduces a related explanation, or a semicolon makes a close relationship clearer. Preserve literal text and tool syntax. |
| C8 — Duplication and misplaced detail | Compare headers, helpers, callers, and documentation. Flag copied rationale that can drift, internal mechanics callers do not need, or obsolete change history. | Retain one authoritative explanation plus a short local warning/reference. Move broad workflow detail out of a local comment. | An implementation detail is part of the caller's contract, or history explains a current restriction. |

Before accepting any subtraction, compare the proposed text with the C1–C4 record. Preserve consequential units, bounds, ownership, failures, invariants, rationale, and distinctions. If a clearer name or small refactor replaces prose, check that it actually communicates those facts.

Comment counts, line ratios, parenthesis counts, and mandatory prose on every trivial operation do not establish compliance with these rules. They may identify candidates for inspection, not prove quality. Tests and builds do not verify prose readability. Review meaning in context.

## Sources and interpretation

Revisited on 2026-10-08, with prose guidance added on 2026-10-09 using these public sources:

- [John Ousterhout, CS 190: Writing Comments (2016)](https://web.stanford.edu/~ouster/cgi-bin/cs190-spring16/lecture.php?topic=comments): interface versus implementation documentation, precise value semantics, placement, avoiding duplication, and writing comments during design.
- [Ousterhout, A Philosophy of Software Design, second-edition extract](https://web.stanford.edu/~ouster/cgi-bin/aposd2ndEdExtract.pdf), section 12.6, printed pages 99–100: explicitly disagrees with treating comments as design failures; interface descriptions reduce the need to inspect implementations. The extract contains selected pages, not the complete commenting chapters.
- [Robert C. Martin, Avoid Redundant Comments](https://www.informit.com/articles/article.aspx?p=1327761): removes commentary that repeats adequately expressed code while retaining a concise behavior description. The publisher's [Clean Code contents](https://www.informit.com/store/clean-code-a-handbook-of-agile-software-craftsmanship-9780132350884) locate the broader discussion in chapter 4; the full chapter was not reviewed here.
- [Martin Fowler, Extract Function](https://refactoring.com/catalog/extractFunction.html): demonstrates replacing a block-label comment with a named operation. Apply when extraction improves understanding, not as a rule to eliminate comments.
- [The Pragmatic Programmer, publisher's tips](https://pragprog.com/tips/), tips 13, 15, and 37: maintain documentation with code, avoid duplicating knowledge, and make contracts explicit.
- [Google developer documentation style guide, Parentheses](https://developers.google.com/style/parentheses): avoid hiding important information in parentheses, question unnecessary asides, and split long interruptions into sentences. The instruction to remove the interruption rather than swap punctuation is this skill's synthesis.
- [Google developer documentation style guide, API reference code comments](https://developers.google.com/style/api-reference-comments): concise first-sentence summaries, relevant usage requirements, and brief member and return descriptions. Its public API coverage requirements are not a mandate to document every internal declaration.
- [MIT/Broad Communication Lab, Coding and Comment Style](https://mitcommlab.mit.edu/broad/commkit/coding-and-comment-style/): communicate through naming, structure, and context before adding prose; tailor comments to their readers and account for maintenance cost.
- [Stack Overflow Blog, Best practices for writing code comments](https://stackoverflow.blog/2021/12/23/best-practices-for-writing-code-comments/): avoid duplicating code or explaining common idioms; explain unusual choices that might otherwise be incorrectly simplified.

The decision table and agent-specific safeguards are this skill's synthesis. Ousterhout's course recommends broad declaration documentation; Martin emphasizes removing redundancy. This skill requires useful contract coverage and follows repository conventions rather than imposing a universal comment quota.
