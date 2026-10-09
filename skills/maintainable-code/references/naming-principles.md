# Naming principles

This is a practical paraphrase of the research in the supplied transcript, centered on Tom Benner, *Naming Things*, second edition (2023). That edition uses **four** principles; older online summaries enumerate seven. The source PDF is not required to use or install this skill.

## Four questions

| Principle | Review question | Typical correction |
|---|---|---|
| Understandability | Can the intended reader identify the domain concept, operation, and relevant contract? | Use the established domain term; expose a surprising effect or a measurement unit. |
| Conciseness | Does every word add meaning at this scope? | Remove redundant context or implementation metadata without obscuring the concept. |
| Consistency | Is this how this language, repository, and bounded context name this concept? | Reuse the established term rather than introducing a new synonym. |
| Distinguishability | Can readers tell genuinely different concepts apart? | Give distinct concepts distinct names and avoid confusable neighbors. |

When these compete, established consistency usually deserves priority. A local preference is rarely worth introducing a second term for the same concept. Identifier length alone is not a quality principle.

## Establish meaning before renaming

- Read implementation and callers. `calculateInvoiceTotal` persisting an invoice is a contract problem; the remedy may be separating persistence or exposing the combined operation explicitly.
- Consult the domain glossary. `customer` (buyer) and `account` (billing relationship) need not be synonyms. Different bounded contexts may intentionally use different vocabulary.
- Match specificity to scope. `i` in a conventional short loop can be clearer than a long invented noun. `data` can fit a generic serializer and still be too vague in a billing API.
- Express units where ambiguity exists. Do not infer milliseconds solely from `timeout = 5000`. A unit-bearing type or established contract can already convey the unit.
- Preserve externally mandated fields and framework method names. Internal naming preferences do not authorize wire-format or database changes.
- Boolean arguments are not universally wrong. Consider call-site readability, named arguments, defaults, and whether separate methods would proliferate without benefit.

## Rename cost

Search symbol references plus public API clients, serialized keys, migrations, reflection, templates, configuration, logs relied on operationally, and documentation. Prefer language-aware rename when available. For a public contract, use an agreed compatibility strategy (for example an alias or versioned migration) instead of a silent breaking rename.

Report a rename only when its likely benefit outweighs compatibility and review cost. Avoid unrelated naming churn.

## Deterministic boundary

Explicit casing conventions, scoped deprecated aliases, or a documented unit suffix can be enforced by an AST/type-aware repository tool. The skill's bundled checker runs those tools; it does not use text regexes to decide whether a name represents the correct business concept.

Length, vague-name matches, confusable names, and suspicious boolean prefixes are review signals unless the project has deliberately adopted a narrow rule with appropriate exceptions.

## Source pointers

Chapter/section references as identified in the supplied transcript: §6.2.3 problem-domain terms; §6.2.7 units; §7.2 abstraction and redundant metadata; §8 consistency; §11.2 trade-offs; §12.4 side effects; §14 controlled vocabulary; §15 renaming; §20.1 length.

Author's book site: <https://www.namingthings.co/>. These notes paraphrase principles; they do not redistribute book text.
