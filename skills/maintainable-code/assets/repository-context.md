# Repository context excerpts

Use only relevant sections. Replace placeholders with discovered or confirmed facts; do not insert this whole template as project instructions.

## Instruction excerpt

For API/domain naming changes, substantial refactors, or maintainability review, use the `maintainable-code` skill. Consult `<glossary path>` for domain terms and `<architecture path>` for agreed boundaries.

Run the project's existing check commands after implementation. Report passed, failed, or incomplete verification accurately; later source changes invalidate a previous pass. Do not change policy to silence failures. Review helpers supply navigation, not quality verdicts.

## Optional glossary structure

| Term | Meaning | Scope | Distinct from / external mapping |
|---|---|---|---|
| `<confirmed term>` | `<project definition>` | `<bounded context>` | `<important distinction>` |

## Optional boundary record

- Boundary: `<source modules> → <forbidden dependency>`
- Rationale: `<real constraint>`
- Enforcement: `<existing architecture check and its configuration>`
- Exceptions: `<specific documented exceptions, if agreed>`
- Compatibility: `<external contracts that must remain stable>`
