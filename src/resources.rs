use clap::{Args, ValueEnum};
use std::collections::BTreeMap;

const FILES: &[(&str, &[u8])] = &[
    (
        "SKILL.md",
        include_bytes!("../skills/maintainable-code/SKILL.md"),
    ),
    (
        "references/naming-principles.md",
        include_bytes!("../skills/maintainable-code/references/naming-principles.md"),
    ),
    (
        "references/commenting-principles.md",
        include_bytes!("../skills/maintainable-code/references/commenting-principles.md"),
    ),
    (
        "references/design-principles.md",
        include_bytes!("../skills/maintainable-code/references/design-principles.md"),
    ),
    (
        "references/review-helpers.md",
        include_bytes!("../skills/maintainable-code/references/review-helpers.md"),
    ),
    (
        "references/updating.md",
        include_bytes!("../skills/maintainable-code/references/updating.md"),
    ),
    (
        "assets/repository-context.md",
        include_bytes!("../skills/maintainable-code/assets/repository-context.md"),
    ),
];

#[derive(Clone, Copy, ValueEnum)]
pub enum Dimension {
    Naming,
    Comments,
    Design,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct RuleOptions {
    /// Rule IDs such as N3 C2 D7.
    #[arg(long, num_args = 1..)]
    rules: Vec<String>,
    #[arg(long, value_enum)]
    dimension: Option<Dimension>,
}

/// Embed only distribution resources, never generated files or source caches.
pub fn package_files() -> BTreeMap<String, Vec<u8>> {
    FILES
        .iter()
        .map(|(path, contents)| ((*path).to_owned(), contents.to_vec()))
        .collect()
}

pub fn rules(options: RuleOptions) -> Result<(String, u8), String> {
    let mut catalog = BTreeMap::new();
    for (prefix, count, file) in [
        ('N', 6, "naming-principles.md"),
        ('C', 8, "commenting-principles.md"),
        ('D', 8, "design-principles.md"),
    ] {
        let path = format!("references/{file}");
        let content = FILES
            .iter()
            .find(|(name, _)| *name == path)
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .ok_or_else(|| format!("Bundled reference missing: {path}"))?;
        for (index, row) in content.lines().enumerate() {
            for number in 1..=count {
                let id = format!("{prefix}{number}");
                if row.starts_with(&format!("| {id} —"))
                    && catalog
                        .insert(id.clone(), (row.to_owned(), path.clone(), index + 1))
                        .is_some()
                {
                    return Err(format!("Duplicate rule: {id}"));
                }
            }
        }
        for number in 1..=count {
            if !catalog.contains_key(&format!("{prefix}{number}")) {
                return Err(format!("Missing bundled rule: {prefix}{number}"));
            }
        }
    }
    let selected = if let Some(dimension) = options.dimension {
        let prefix = match dimension {
            Dimension::Naming => 'N',
            Dimension::Comments => 'C',
            Dimension::Design => 'D',
        };
        catalog
            .keys()
            .filter(|id| id.starts_with(prefix))
            .cloned()
            .collect()
    } else {
        options.rules
    };
    let mut output = String::from(
        "Review guidance, not detected violations. Inspect evidence and exceptions before reporting.\n| Rule / principle | Inspect and flag when | Correction / example | Keep when |\n|---|---|---|---|\n",
    );
    let mut seen = std::collections::HashSet::new();
    for id in selected {
        let (row, path, line) = catalog
            .get(&id)
            .ok_or_else(|| format!("Unknown rule: {id}"))?;
        if seen.insert(id) {
            output.push_str(&format!("{row}\n<!-- {path}:{line} -->\n"));
        }
    }
    Ok((output, 0))
}
