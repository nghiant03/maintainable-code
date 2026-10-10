# Naming principles

This is a practical paraphrase of the research in the supplied transcript, centered on Tom Benner, *Naming Things*, second edition (2023). That edition uses **four** principles; older online summaries enumerate seven. The source PDF is not required to use or install this skill.

## Concrete naming rules

For each candidate, read its declaration, implementation, and at least one use. Flag a name only when those locations show a misleading or missing distinction. The examples below are illustrative, not project vocabulary.

| Rule / principle | Inspect and flag when | Correction / example | Keep when |
|---|---|---|---|
| N1 — Understandability: domain meaning | Compare the value's role with its name and the glossary. Flag a generic name when a reader must inspect its producer to discover the domain concept. | Rename billing API `data` to `invoiceLines` if it holds invoice lines. | `data` represents arbitrary payloads in a generic serializer; `i` is a short loop index. |
| N2 — Understandability: operation and effects | Trace mutation, persistence, and I/O. Flag an operation whose name or documented contract implies calculation or lookup but also performs an unexpected write. | Separate persistence from `calculateInvoiceTotal`, or expose the combined operation as `calculateAndSaveInvoice` with a documented write contract. | The established operation name and interface contract already communicate the effect. |
| N3 — Understandability: units and conditions | Read arithmetic, comparisons, defaults, and call sites. Flag an ambiguous unit or boolean condition when the declaration and contract do not resolve it. | Use `timeoutMs` for a millisecond scalar; replace ambiguous `setMode(true)` with a named argument or explicit operation. | A duration type supplies the unit; `setEnabled(true)` already states the condition. |
| N4 — Conciseness | Remove each candidate word mentally. Flag redundant scope or metadata only if the shorter name retains the same meaning and distinction. | Use `invoice.total` rather than `invoice.invoiceTotalAmountValue` when the fields express the same contract. | Removing a word loses a unit, domain distinction, or established public spelling. |
| N5 — Consistency | Compare declarations and callers within the same bounded context. Flag different terms for the same concept without an established distinction. | Use the glossary's `customer` instead of introducing `client` for the same buyer. | A framework fixes the spelling, or another bounded context intentionally uses a different term. |
| N6 — Distinguishability | Compare names used together and trace their values. Flag near-identical names that conceal different states, roles, or contracts. | Replace `invoice1` and `invoice2` with `draftInvoice` and `postedInvoice` when those are their roles. | Positional names are the contract, as in generic pair processing. |

When these rules compete, retain the established term unless it misrepresents the concept or hides a consequential distinction. Do not introduce a second term solely for stylistic reasons. Identifier length alone is not a quality principle.

## Establish meaning before renaming

- Read implementation and callers. `calculateInvoiceTotal` persisting an invoice is a contract problem; the remedy may be separating persistence or exposing the combined operation explicitly.
- Consult the domain glossary. `customer` (buyer) and `account` (billing relationship) need not be synonyms. Different bounded contexts may intentionally use different vocabulary.
- Match specificity to scope. `i` in a conventional short loop can be clearer than a long invented noun. `data` can fit a generic serializer and still be too vague in a billing API.
- Express units where ambiguity exists. Do not infer milliseconds solely from `timeout = 5000`. A unit-bearing type or established contract can already convey the unit.
- Preserve externally mandated fields and framework method names. Internal naming corrections do not authorize wire-format or database changes.
- Boolean arguments are not universally wrong. Consider call-site readability, named arguments, defaults, and whether separate methods would proliferate without benefit.

## Rename cost

Search symbol references plus public API clients, serialized keys, migrations, reflection, templates, configuration, logs relied on operationally, and documentation. Use language-aware rename when available; otherwise update references explicitly and verify them with search and relevant checks. Neither method replaces inspection of non-symbol consumers. For a public contract, use an agreed compatibility strategy (for example an alias or versioned migration) instead of a silent breaking rename.

Report a rename only when its likely benefit outweighs compatibility and review cost. Avoid unrelated naming churn.

## Deterministic boundary

Explicit casing conventions, scoped deprecated aliases, or a documented unit suffix can be enforced by an AST/type-aware repository tool. Run those tools directly. The skill's source inventory helps locate declarations; it does not decide whether a name represents the correct business concept.

Length, vague-name matches, confusable names, and suspicious boolean prefixes are review signals unless the project has deliberately adopted a narrow rule with appropriate exceptions.

## Source pointers

Chapter/section references as identified in the supplied transcript: §6.2.3 problem-domain terms; §6.2.7 units; §7.2 abstraction and redundant metadata; §8 consistency; §11.2 trade-offs; §12.4 side effects; §14 controlled vocabulary; §15 renaming; §20.1 length.

Author's book site: <https://www.namingthings.co/>. These notes paraphrase principles; they do not redistribute book text.
