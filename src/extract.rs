use clap::ValueEnum;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Symbols,
    Comments,
    All,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    Javascript,
    Typescript,
    Tsx,
    Go,
    Rust,
    C,
    Cpp,
    Java,
    CSharp,
    Ruby,
    Bash,
    Html,
    Css,
    Yaml,
    Toml,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Javascript => "javascript",
            Self::Typescript => "typescript",
            Self::Tsx => "tsx",
            Self::Go => "go",
            Self::Rust => "rust",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::CSharp => "csharp",
            Self::Ruby => "ruby",
            Self::Bash => "bash",
            Self::Html => "html",
            Self::Css => "css",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
        }
    }

    fn grammar(self) -> tree_sitter::Language {
        match self {
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Javascript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::Html => tree_sitter_html::LANGUAGE.into(),
            Self::Css => tree_sitter_css::LANGUAGE.into(),
            Self::Yaml => tree_sitter_yaml::LANGUAGE.into(),
            Self::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
        }
    }

    fn declaration(self, kind: &str) -> bool {
        let kinds: &[&str] = match self {
            Self::Python => &[
                "function_definition",
                "class_definition",
                "assignment",
                "named_expression",
                "for_statement",
                "default_parameter",
                "typed_default_parameter",
            ],
            Self::Javascript => &[
                "function_declaration",
                "generator_function_declaration",
                "class_declaration",
                "method_definition",
                "variable_declarator",
                "field_definition",
            ],
            Self::Typescript | Self::Tsx => &[
                "function_declaration",
                "generator_function_declaration",
                "class_declaration",
                "method_definition",
                "method_signature",
                "variable_declarator",
                "interface_declaration",
                "type_alias_declaration",
                "enum_declaration",
                "public_field_definition",
                "property_signature",
            ],
            Self::Go => &[
                "function_declaration",
                "method_declaration",
                "type_spec",
                "var_spec",
                "const_spec",
                "short_var_declaration",
                "field_declaration",
                "parameter_declaration",
            ],
            Self::Rust => &[
                "function_item",
                "struct_item",
                "enum_item",
                "trait_item",
                "type_item",
                "const_item",
                "static_item",
                "mod_item",
                "field_declaration",
                "enum_variant",
                "let_declaration",
                "parameter",
            ],
            Self::C => &[
                "function_definition",
                "struct_specifier",
                "union_specifier",
                "enum_specifier",
                "enumerator",
                "declaration",
                "field_declaration",
                "parameter_declaration",
                "type_definition",
            ],
            Self::Cpp => &[
                "function_definition",
                "struct_specifier",
                "union_specifier",
                "enum_specifier",
                "enumerator",
                "declaration",
                "field_declaration",
                "parameter_declaration",
                "type_definition",
                "class_specifier",
                "namespace_definition",
                "alias_declaration",
            ],
            Self::Java => &[
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
                "record_declaration",
                "method_declaration",
                "constructor_declaration",
                "variable_declarator",
                "formal_parameter",
                "enum_constant",
            ],
            Self::CSharp => &[
                "class_declaration",
                "interface_declaration",
                "struct_declaration",
                "enum_declaration",
                "record_declaration",
                "method_declaration",
                "constructor_declaration",
                "variable_declarator",
                "property_declaration",
                "parameter",
                "enum_member_declaration",
            ],
            Self::Ruby => &[
                "class",
                "module",
                "method",
                "singleton_method",
                "assignment",
            ],
            Self::Bash => &["function_definition", "variable_assignment"],
            Self::Html | Self::Css | Self::Yaml | Self::Toml => &[],
        };
        kinds.contains(&kind)
    }
}

pub fn language_for(path: &Path) -> Option<Language> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    Some(match extension.as_str() {
        "py" | "pyi" => Language::Python,
        "js" | "jsx" | "mjs" | "cjs" => Language::Javascript,
        "ts" => Language::Typescript,
        "tsx" => Language::Tsx,
        "go" => Language::Go,
        "rs" => Language::Rust,
        "c" | "h" => Language::C,
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => Language::Cpp,
        "java" => Language::Java,
        "cs" => Language::CSharp,
        "rb" => Language::Ruby,
        "sh" | "bash" => Language::Bash,
        "html" | "htm" => Language::Html,
        "css" => Language::Css,
        "yaml" | "yml" => Language::Yaml,
        "toml" => Language::Toml,
        _ => return None,
    })
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

/// Use binding fields, not initializer values; descend through C declarators.
fn names(node: Node<'_>) -> Vec<Node<'_>> {
    for field in ["name", "pattern", "left"] {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children_by_field_name(field, &mut cursor).collect();
        if !children.is_empty() {
            return children;
        }
    }
    let mut cursor = node.walk();
    node.children_by_field_name("declarator", &mut cursor)
        .map(|mut child| {
            while let Some(nested) = child.child_by_field_name("declarator") {
                child = nested;
            }
            child
        })
        .collect()
}

/// Python documentation is the first non-comment statement in a module or
/// function/class body. Preserve literal spelling rather than interpreting escapes.
fn docstring(node: Node<'_>) -> bool {
    if node.kind() != "expression_statement" {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    let valid_parent = parent.kind() == "module"
        || (parent.kind() == "block"
            && parent.parent().is_some_and(|owner| {
                matches!(owner.kind(), "function_definition" | "class_definition")
            }));
    if !valid_parent {
        return false;
    }
    let mut cursor = parent.walk();
    let first = parent
        .named_children(&mut cursor)
        .find(|child| child.kind() != "comment");
    first == Some(node)
        && node
            .named_child(0)
            .is_some_and(|child| matches!(child.kind(), "string" | "concatenated_string"))
}

fn python_parameter(node: Node<'_>) -> bool {
    node.kind() == "identifier"
        && node.parent().is_some_and(|parent| {
            parent.kind() == "parameters"
                || (matches!(
                    parent.kind(),
                    "typed_parameter" | "list_splat_pattern" | "dictionary_splat_pattern"
                ) && parent.named_child(0) == Some(node))
        })
}

fn documentation_literal(node: Node<'_>, source: &str) -> bool {
    let mut stack = vec![node];
    while let Some(literal) = stack.pop() {
        if literal.kind() == "string" {
            let prefix = text(literal, source)
                .split(['\'', '"'])
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if prefix.contains('f') || prefix.contains('b') {
                return false;
            }
        } else {
            let mut cursor = literal.walk();
            stack.extend(literal.named_children(&mut cursor));
        }
    }
    true
}

pub fn records(
    source: &str,
    language: Language,
    mode: Mode,
) -> Result<(Vec<Value>, Vec<String>), String> {
    extract(source, language, mode, 0, 0)
}

fn extract(
    source: &str,
    language: Language,
    mode: Mode,
    line_offset: usize,
    column_offset: usize,
) -> Result<(Vec<Value>, Vec<String>), String> {
    if mode == Mode::Symbols && matches!(language, Language::Css | Language::Yaml | Language::Toml)
    {
        return Ok((
            vec![],
            vec![format!(
                "Symbol extraction is not configured for {}; comments are supported",
                language.name()
            )],
        ));
    }
    let mut parser = Parser::new();
    parser
        .set_language(&language.grammar())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(source, None)
        .ok_or("Parser did not produce a syntax tree")?;
    let mut problems = Vec::new();
    if tree.root_node().has_error() {
        problems.push(format!(
            "{} syntax tree contains parse errors; extraction may be incomplete",
            language.name()
        ));
    }
    let mut output = Vec::new();
    let mut stack = vec![(tree.root_node(), String::new())];
    while let Some((node, scope)) = stack.pop() {
        let point = node.start_position();
        let mut record = json!({"line": point.row + 1 + line_offset,
            "column": point.column + 1 + if point.row == 0 { column_offset } else { 0 },
            "endLine": node.end_position().row + 1 + line_offset});
        if mode != Mode::Symbols && (node.kind() == "comment" || node.kind().ends_with("_comment"))
        {
            record["kind"] = json!("comment");
            record["text"] = json!(text(node, source));
            output.push(record);
            continue;
        }
        if language == Language::Python && mode != Mode::Symbols && docstring(node) {
            // Every part of a concatenated literal must be a plain string.
            if documentation_literal(node, source) {
                let mut doc = record.clone();
                doc["kind"] = json!("docstring");
                doc["text"] = json!(text(node, source));
                output.push(doc);
            }
        }
        let mut child_scope = scope.clone();
        let declarations = if language == Language::Python && python_parameter(node) {
            vec![node]
        } else if language.declaration(node.kind()) {
            names(node)
        } else {
            vec![]
        };
        for name_node in declarations {
            let name = text(name_node, source);
            let qualified = if language == Language::Python && !scope.is_empty() {
                format!("{scope}.{name}")
            } else {
                name.to_owned()
            };
            if language == Language::Python
                && matches!(node.kind(), "function_definition" | "class_definition")
            {
                child_scope = qualified.clone();
            }
            if mode != Mode::Comments {
                let mut declaration = record.clone();
                declaration["kind"] = json!(if language == Language::Python {
                    match node.kind() {
                        "function_definition" => "function",
                        "class_definition" => "class",
                        "identifier" => "parameter",
                        _ => "binding",
                    }
                } else {
                    node.kind()
                });
                declaration["name"] = json!(qualified);
                if language == Language::Python && node.kind() == "function_definition" {
                    declaration["parameters"] = node
                        .child_by_field_name("parameters")
                        .map(|node| json!(text(node, source)))
                        .unwrap_or(Value::Null);
                    declaration["returns"] = node
                        .child_by_field_name("return_type")
                        .map(|node| json!(text(node, source)))
                        .unwrap_or(Value::Null);
                }
                output.push(declaration);
            }
        }
        // HTML exposes script/style contents as opaque raw_text. Parse those
        // bodies separately and translate their byte positions back into the file.
        if language == Language::Html && node.kind() == "raw_text" {
            let embedded = node.parent().and_then(|parent| match parent.kind() {
                "script_element" => Some(Language::Javascript),
                "style_element" => Some(Language::Css),
                _ => None,
            });
            if let Some(embedded) = embedded
                && (mode != Mode::Symbols || embedded == Language::Javascript)
            {
                let (nested, errors) = extract(
                    text(node, source),
                    embedded,
                    mode,
                    line_offset + point.row,
                    point.column + if point.row == 0 { column_offset } else { 0 },
                )?;
                output.extend(nested);
                problems.extend(errors);
            }
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        stack.extend(
            children
                .into_iter()
                .rev()
                .map(|child| (child, child_scope.clone())),
        );
    }
    output.sort_by_key(|record| {
        (
            record["line"].as_u64().unwrap_or(0),
            record["column"].as_u64().unwrap_or(0),
        )
    });
    Ok((output, problems))
}
