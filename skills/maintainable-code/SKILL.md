---
name: maintainable-code
description: Improve code naming, comments, API contracts, and module responsibilities. Use for maintainability reviews, comment and docstring audits, API or domain renames, substantial refactors, and repository quality-check or context setup. Includes a checker for project-defined rules. Not for formatting-only edits or unrelated prose.
compatibility: File access, code search, and command execution. Bundled checker requires Python 3.11+ and the repository's configured tools.
user-invocable: true
---

# Maintainable Code

Make the next change easier. Prefer concrete evidence and small useful corrections over generic clean-code advice.

The skill supplies a reusable checker and review guidance. The repository owns its vocabulary, architecture, commands, and exceptions. The harness supplies file access and execution; ordinary skill loading is sufficient.

Use existing repository context and tools by default. `quality.toml` is optional for the skill. Create it only when the user explicitly requests bundled checker setup; otherwise run existing project checks directly.

## Choose the workflow

| Request | Workflow |
|---|---|
| Initialize this repository / set up quality checks | Initialize |
| Run maintainability checks | Check |
| Review names, APIs, comments, or design | Review (read-only unless edits requested) |
| Implement or refactor something | Review relevant context, Implement, then Check and coverage gate |
| Revise glossary, boundaries, or verification policy | Update context |

Use the user's requested scope. Resolve the actual project/worktree directory separately from this skill's installation directory. Supporting paths below are relative to the directory containing this `SKILL.md`; resolve that directory from the harness, rather than assuming the current directory contains the checker.

A general maintainability or code-quality review covers **identifier clarity, API contracts, comment correctness/coverage, comment readability, and design/responsibilities**. Assess how names and prose help a reader understand working code, as well as whether behavior is correct. An explicitly narrow request limits the dimensions reviewed; record exclusions rather than silently dropping dimensions. Check-only and initialization requests do not imply a full semantic review.

## Initialize

1. Read applicable project instructions and contributor docs. Inspect manifests, workspace/package boundaries, CI, existing formatting/lint/type/test configuration, architecture notes, domain vocabulary, and a small representative code sample.
2. Summarize the facts you can establish: installed tools, existing check commands, conventions, documented contracts. Distinguish these from proposed policy. Repeated imports do not prove intended architecture.
3. Ask focused questions only about consequential unknowns: ambiguous domain terms, forbidden dependency directions, required versus advisory checks, compatibility promises, or acceptance of existing debt. Continue independent discovery while awaiting answers. Reuse explicit decisions already documented.
4. Create only the missing useful context:
   - A repository `quality.toml` only when bundled checker setup is explicitly requested, with commands verified from this repository. Read [checker configuration](references/checker.md); adapt [the template](assets/quality.toml.example), rather than copying illustrative commands blindly.
   - A few lines in the repository's instruction file describing actual check commands and relevant policy locations. Preserve existing instructions. `AGENTS.md` is common; use the harness's actual instruction mechanism (for example `CLAUDE.md` for Claude Code).
   - A glossary only where terminology needs clarification. Use [the context template](assets/repository-context.md) selectively.
   - Executable architecture constraints only for agreed boundaries, using existing repository tools. Run them directly or through the bundled checker's `commands`; it has no built-in language-specific rules.
5. Run existing project checks directly, or use the bundled checker if its policy is configured. Report what was reused, added, passed, failed, and left uncertain. If no runnable checks are found or tools are unavailable, report the verification gap.

Initialization is an agent workflow, not an automatic policy generator. Do not invent a layering scheme, install dependencies without task authorization, or overwrite existing policy. If debt needs baselining, use the project's existing tool after explicit agreement; this bundle does not generate baselines.

## Review

Review is read-only unless edits are requested. Track these checkpoints separately, using an inline checklist or the harness's task tracker. Record naming and comment observations as each file or review unit is inspected, before extended defect investigations consume the review. For a long review, retain the scope and evidence in task-local notes so the review can resume without losing coverage. When delegation is already authorized, assign each included dimension explicitly and request the same evidence in reviewer handoffs.

```text
[ ] Establish scope and read applicable references
[ ] Assess identifier clarity and API contracts
[ ] Comments pass 1: correctness and contract coverage
[ ] Comments pass 2: clarity and subtraction
[ ] Assess design and responsibilities
[ ] Apply the coverage gate
```

1. **Scope and references.** Establish scope from the requested files or diff, including relevant new files. For a repository review, identify the inspected files and whether coverage is exhaustive or sampled; do not silently substitute a diff review. Read code, nearby conventions, callers, tests, and relevant glossary entries. Follow dependencies beyond the scope when needed to understand a contract. Read [naming principles](references/naming-principles.md), [commenting principles](references/commenting-principles.md), and [design principles](references/design-principles.md) for a general review; for a narrow review, read the references for its included dimensions.
2. **Naming and API contracts.** Read declarations and uses, including internal helpers and local names, and compare the concept a name suggests with its actual role. Evaluate **understandability, conciseness, consistency, and distinguishability** against domain vocabulary and nearby conventions. For a finding, record the identifier, the reader's likely interpretation, the actual meaning, and a clearer name or contract correction with its compatibility cost. Correct units or consistent public fields do not establish clarity of internal names; type and validation defects do not substitute for this assessment.
3. **Comments pass 1 — correctness and contract coverage.** Verify claims against code, tests, specifications, or recorded decisions. Identify missing information needed to use or change the code safely, including interfaces with no comments. Record consequential contracts and rationale that subsequent shortening must preserve. Apply this pass to the selected scope, not only newly added comments; preserve required notices and tool directives.
4. **Comments pass 2 — clarity and subtraction.** After pass 1, revisit the same scope and read comments as prose. Identify redundant narration, interrupting asides, duplicated explanations, and misplaced detail. A true comment can still be hard to read, so perform this pass even when pass 1 finds no inaccuracies. In a read-only review, suggest corrections rather than editing. Check proposed shortening against pass 1 so it retains consequential units, bounds, ownership, failures, invariants, and rationale. If there are no comments, record that observation for pass 2; it does not excuse missing-contract assessment in pass 1.
5. **Design and responsibilities.** Look for information leakage, surprising side effects, responsibilities that change for unrelated reasons, duplicated business knowledge, and abstractions that force callers to understand more rather than less. Record the interfaces or boundaries inspected and the evidence for any finding.
6. **Coverage gate.** Reconcile the checkpoints with the coverage record below before reporting completion. Report supported findings even if coverage remains incomplete, but make that limitation explicit.

## Implement

1. Review the relevant context using the checkpoints above. For implementation or refactoring, cover naming, both comment passes, and design in the affected scope; this is not a request to audit unrelated repository code.
2. Implement only authorized changes, incrementally. Preserve behavior unless a behavior change is requested. For renames, inspect external API fields, serialization, database columns, reflection, configuration, docs, and consumers. A successful symbol rename alone does not establish compatibility. Preserve externally fixed names or propose a deliberate migration.
3. Document consequential contracts and non-obvious decisions using the commenting principles. Keep coherent routines; splitting merely to hit a length threshold can increase complexity.
4. After a coherent edit batch, run Check and examine the final diff. Refresh the affected review checkpoints on the final source state, including both comment passes and confirmation that prose edits preserved the contracts identified in pass 1. Apply the coverage gate before claiming implementation complete; a passing test command does not complete semantic review.

Do not impose universal bans on `data`, `manager`, boolean arguments, short local variables, or long functions. Framework names, external schemas, generic infrastructure, and bounded contexts need contextual treatment. No numerical maintainability score.

## Coverage gate

For Review and Implement, keep a compact coverage record and include it in the final report. Each included dimension needs inspected file paths, symbols, or line ranges and a result: **findings**, **no supported issues**, or **incomplete**. Point **findings** entries to the actual findings; support **no supported issues** with a representative assessed name or comment and why it is appropriate. Use **outside requested scope** only with a reason tied to the user's request. Shared locations can reference the scope line; do not enumerate every comment or force findings to fill the record.

```text
Scope: <files/diff/sample; exclusions and limits>
Naming/API: <identifier clarity and contract results; inspected locations>
Comments — correctness/coverage: <result; locations and missing-contract assessment>
Comments — clarity/subtraction: <result; locations revisited after pass 1>
Design/responsibilities: <result; inspected locations or scope reference>
```

Before claiming completion, check that every included checkpoint has evidence for the current source state and that both comment passes cover the same selected scope. If a checkpoint is missing, perform it or report the review as incomplete with the specific gap. A narrower inspected sample can be complete for that sample, not for the whole repository. Later relevant edits invalidate the affected checkpoints.

This gate checks accountable coverage, not semantic quality by itself. A checked box, subagent success report, passing tests, or bundled checker exit code is not evidence that naming or prose was assessed. Do not invent defects or claim unobserved work to satisfy the gate.

## Check

Discover check commands from existing project instructions, manifests, tool configuration, and CI. Run the relevant check-only commands directly; no new configuration is needed. Report the commands and their results. If none can be established, report that verification is incomplete rather than creating policy.

When the repository already has a policy for the bundled checker, run it using the resolved skill location and explicit checkout:

```sh
python3 /absolute/path/to/maintainable-code/scripts/check.py --project /absolute/path/to/project --mode fast
python3 /absolute/path/to/maintainable-code/scripts/check.py --project /absolute/path/to/project --mode full --format json
```

- For the bundled checker, read `quality.toml` and [the checker contract](references/checker.md) first. Commands execute in the configured repository working directory, using installed tools. The checker never installs tools, reformats code, or revises policy itself; configured commands must be check-only.
- Use fast checks while iterating and the project's required full checks before declaring implementation complete. Do not run commands just to answer a read-only design question.
- Bundled checker exit `0` means selected checks passed (warnings may remain); `1` means violations; `2` means verification incomplete. For direct commands, interpret each tool's exit codes. Inspect individual results; incomplete verification can coexist with concrete failures.
- Correct failures you can address within scope, then rerun affected verification. Stop repeated repairs when no progress is made; report the unresolved issue. Do not delete tests, weaken severity, add exclusions, or expand a baseline to manufacture a pass.
- Results describe the selected source state, not a permanent certification. Relevant later edits invalidate a pass. The checker detects differing before/after snapshots; it is not an atomic snapshot or an environment cache.
- Missing `quality.toml` is not a setup requirement for the skill. Use existing project checks directly. Do not substitute an empty check run for success.

## Update context

Treat policy changes as deliberate work. Inspect current policy, its rationale, callers, and tooling. Explain the changed agreement, preserve unrelated content, then validate the revised configuration. Move an observed convention into policy only when it reflects an actual project decision. Keep exceptions narrow and justified; never update policy implicitly during Check.

## Report

Scale the response to the task. Organize a general review into visible results for **Naming/API**, **Comments — correctness/coverage**, **Comments — clarity/subtraction**, and **Design/responsibilities**, alongside other requested criteria. Use sections for a detailed report or labeled bullets for a short answer. A coverage table alone is not the assessment. Report supported findings in this form:

```text
path:line — finding
Evidence and maintenance consequence.
Smallest useful correction; compatibility implications if relevant.
```

For implementation or initialization, summarize changes and verification: mode, passed/failed/incomplete, remaining warnings or blockers, and what was not checked. Do not describe a fast pass as a full pass or a deterministic rule as proof of semantic quality. If no concrete issue is found, say so with the scope inspected.

For all reviews and implementations, include the coverage record above. Passing tests or enumerating every comment does not establish writing quality; do not claim a repository-wide audit after reviewing only a diff.

Before finalizing, reconcile findings from task notes and authorized reviewers dimension by dimension. Validate evidence, include or group supported findings with representative locations and consequences, and record a reason for omissions. Disclose scope exclusions and unresolved gaps. Rank urgent defects first, but retain supported naming and prose corrections in both the summary and remediation order; use a precise section reference when detail lives elsewhere. Severity determines order, not whether a requested dimension is reported. A general link to a report or praise such as “mostly sound naming” does not convey outstanding corrections.

For comment-readability findings, include a precise location, a short representative excerpt, the reader burden, and the smallest useful correction. Verify that the correction preserves the contracts identified in pass 1. For example, explain how repeated rationale can drift or how an interrupting aside obscures a condition; punctuation or length alone is not a defect. Group recurring patterns rather than enumerate every occurrence. A full before/after rewrite is optional, not a requirement to edit during a read-only review.

## Resources

- [Naming principles](references/naming-principles.md): four principles and rename trade-offs.
- [Design principles](references/design-principles.md): balanced guidance from the books.
- [Commenting principles](references/commenting-principles.md): when to comment, what to explain, placement, and maintenance.
- [Checker contract](references/checker.md): configuration, output, scope, and limitations.
- [Repository context template](assets/repository-context.md): optional instruction/glossary excerpts.
- [Quality template](assets/quality.toml.example): configure only applicable checks.
