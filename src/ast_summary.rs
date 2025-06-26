use std::path::Path;
use tree_sitter::{Language, Parser, TreeCursor};
use tree_sitter_rust::LANGUAGE as RUST;
use tree_sitter_javascript::LANGUAGE as JAVASCRIPT;
use tree_sitter_python::LANGUAGE as PYTHON;

fn parser_for_extension(ext: &str) -> Option<Language> {
    match ext {
        "rs" => Some(RUST.into()),
        "js" | "jsx" | "ts" | "tsx" => Some(JAVASCRIPT.into()),
        "py" => Some(PYTHON.into()),
        _ => None,
    }
}

fn summarize_nodes(cursor: &mut TreeCursor, source: &str, summary: &mut String) {
    let node = cursor.node();
    match node.kind() {
        "function_item" | "function_declaration" | "method_definition" | "function_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                if let Ok(txt) = name.utf8_text(source.as_bytes()) {
                    summary.push_str(&format!("fn {}\n", txt));
                }
            }
        }
        "struct_item" | "class_declaration" | "class_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                if let Ok(txt) = name.utf8_text(source.as_bytes()) {
                    summary.push_str(&format!("class {}\n", txt));
                }
            }
        }
        "enum_item" => {
            if let Some(name) = node.child_by_field_name("name") {
                if let Ok(txt) = name.utf8_text(source.as_bytes()) {
                    summary.push_str(&format!("enum {}\n", txt));
                }
            }
        }
        _ => {}
    }
    if cursor.goto_first_child() {
        loop {
            summarize_nodes(cursor, source, summary);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

pub fn summarize(path: &Path, content: &str) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    let language = parser_for_extension(ext)?;
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(content, None)?;
    let mut cursor = tree.root_node().walk();
    let mut summary = String::new();
    summarize_nodes(&mut cursor, content, &mut summary);
    if summary.is_empty() {
        None
    } else {
        Some(summary)
    }
}
