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
| Implement or refactor something | Review relevant context, implement, then Check |
| Revise glossary, boundaries, or verification policy | Update context |

Use the user's requested scope. Resolve the actual project/worktree directory separately from this skill's installation directory. Supporting paths below are relative to the directory containing this `SKILL.md`; resolve that directory from the harness, rather than assuming the current directory contains the checker.

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

## Review and implement

1. Establish scope from the requested files or diff, including relevant new files. Read changed code, nearby conventions, callers, tests, and relevant glossary entries. Follow dependencies beyond the diff when needed to understand a contract.
2. Establish actual behavior before judging its name. Consult [naming principles](references/naming-principles.md) for naming/API work and [design principles](references/design-principles.md) for responsibility or boundary changes.
3. Evaluate names through **understandability, conciseness, consistency, and distinguishability**. Use existing domain terminology. Make meaningful side effects and units discoverable in the name or contract, considering unit-bearing types and local conventions.
4. Look for information leakage, surprising side effects, responsibilities that change for unrelated reasons, duplicated business knowledge, and abstractions that force callers to understand more rather than less.
5. For renames, inspect external API fields, serialization, database columns, reflection, configuration, docs, and consumers. A successful symbol rename alone does not establish compatibility. Preserve externally fixed names or propose a deliberate migration.
6. Implement only authorized changes, incrementally. Preserve behavior unless a behavior change is requested. Use [commenting principles](references/commenting-principles.md) when documenting contracts, explaining non-obvious decisions, or reviewing comments. Keep coherent routines; splitting merely to hit a length threshold can increase complexity.
7. Review comments in two passes: **correctness and contract coverage**, then **clarity and subtraction**. Verify claims first; then remove redundant narration, interrupting asides, duplicated explanations, and misplaced detail without losing consequential contracts or rationale. Read the result as prose, not just a list of true statements. Apply both passes to the requested scope, not only newly added comments.
8. After a coherent edit batch, run Check. Examine the final diff and report concise findings with suggested corrections or a completion summary.

Do not impose universal bans on `data`, `manager`, boolean arguments, short local variables, or long functions. Framework names, external schemas, generic infrastructure, and bounded contexts need contextual treatment. No numerical maintainability score.

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

Scale the response to the task. For a review, report only supported findings:

```text
path:line — finding
Evidence and maintenance consequence.
Smallest useful correction; compatibility implications if relevant.
```

For implementation or initialization, summarize changes and verification: mode, passed/failed/incomplete, remaining warnings or blockers, and what was not checked. Do not describe a fast pass as a full pass or a deterministic rule as proof of semantic quality. If no concrete issue is found, say so with the scope inspected.

For comment reviews, state the coverage and distinguish factual corrections from readability findings. Passing tests or enumerating every comment does not establish writing quality; do not claim a repository-wide audit after reviewing only a diff.

## Resources

- [Naming principles](references/naming-principles.md): four principles and rename trade-offs.
- [Design principles](references/design-principles.md): balanced guidance from the books.
- [Commenting principles](references/commenting-principles.md): when to comment, what to explain, placement, and maintenance.
- [Checker contract](references/checker.md): configuration, output, scope, and limitations.
- [Repository context template](assets/repository-context.md): optional instruction/glossary excerpts.
- [Quality template](assets/quality.toml.example): configure only applicable checks.
