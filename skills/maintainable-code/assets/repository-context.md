# Repository context excerpts

Use only relevant sections. Replace placeholders with discovered or confirmed facts; do not insert this whole template as project instructions.

## Instruction excerpt

For API/domain naming changes, substantial refactors, or maintainability review, use the `maintainable-code` skill. Consult `<glossary path>` for domain terms and `<architecture path>` for agreed boundaries.

Run the project's existing check commands after implementation. If the project uses the bundled checker, run `python3 <installed-or-vendored-skill-path>/scripts/check.py --project <checkout-path> --mode full`. Report passed, failed, or incomplete verification accurately; later source changes invalidate a previous pass. Do not change policy to silence failures.

## Optional glossary structure

| Term | Meaning | Scope | Distinct from / external mapping |
|---|---|---|---|
| `<confirmed term>` | `<project definition>` | `<bounded context>` | `<important distinction>` |

## Optional boundary record

- Boundary: `<source modules> → <forbidden dependency>`
- Rationale: `<real constraint>`
- Enforcement: `<quality.toml rule ID or existing architecture check>`
- Exceptions: `<specific documented exceptions, if agreed>`
- Compatibility: `<external contracts that must remain stable>`
