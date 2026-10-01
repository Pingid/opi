//! Step 2: the tree laid out as block YAML, docs as `#` comments.

use core::fmt::Write;

use super::tree::{Entry, Node, Table};
use super::{SerializeOptions, yaml_key};

pub(super) fn render(root: &Table, options: &SerializeOptions) -> String {
    let mut out = String::new();
    for line in &options.header {
        writeln!(out, "# {line}").unwrap();
    }
    if options.docs && !root.doc.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        write_doc(&mut out, &root.doc, 0);
    }
    if !out.is_empty() {
        out.push('\n');
    }
    write_entries(&mut out, &root.entries, 0, options.docs);
    out
}

/// `key: value` lines at `indent`, with a blank line before each documented
/// key so the comments read as belonging to the line below them.
fn write_entries(out: &mut String, entries: &[Entry], indent: usize, docs: bool) {
    for (i, entry) in entries.iter().enumerate() {
        let doc = entry_doc(entry);
        if docs && !doc.is_empty() {
            if i > 0 {
                out.push('\n');
            }
            write_doc(out, doc, indent);
        }
        write_entry(out, entry, indent, docs);
    }
}

/// Field docs, else the docs of the table's (or element's) type.
fn entry_doc(entry: &Entry) -> &[String] {
    match &entry.value {
        _ if !entry.doc.is_empty() => &entry.doc,
        Node::Table(t) => &t.doc,
        Node::Array(items) => match items.first() {
            Some(Node::Table(t)) => &t.doc,
            _ => &entry.doc,
        },
        Node::Scalar(_) => &entry.doc,
    }
}

fn write_entry(out: &mut String, entry: &Entry, indent: usize, docs: bool) {
    let pad = " ".repeat(indent);
    let key = yaml_key(&entry.key);
    match &entry.value {
        node if is_inline(node) => writeln!(out, "{pad}{key}: {}", inline(node)).unwrap(),
        Node::Table(table) => {
            writeln!(out, "{pad}{key}:").unwrap();
            write_entries(out, &table.entries, indent + 2, docs);
        }
        Node::Array(items) => {
            writeln!(out, "{pad}{key}:").unwrap();
            for (i, item) in items.iter().enumerate() {
                // Field docs once, on the first element, not on every one.
                write_item(out, item, indent + 2, docs && i == 0);
            }
        }
        Node::Scalar(_) => unreachable!("scalars are inline"),
    }
}

/// `- item`. A table's first key goes on the `- ` line, with its docs above
/// it.
fn write_item(out: &mut String, item: &Node, indent: usize, docs: bool) {
    let pad = " ".repeat(indent);
    match item {
        Node::Table(table) if !is_inline(item) => {
            let (first, rest) = table.entries.split_first().expect("non-empty");
            let mut first_lines = String::new();
            write_entry(&mut first_lines, first, indent + 2, docs);
            if docs && !first.doc.is_empty() {
                write_doc(out, &first.doc, indent);
            }
            out.push_str(&pad);
            out.push_str("- ");
            out.push_str(&first_lines[indent + 2..]);
            if !rest.is_empty() {
                if docs {
                    out.push('\n');
                }
                write_entries(out, rest, indent + 2, docs);
            }
        }
        Node::Array(items) if !is_inline(item) => {
            writeln!(out, "{pad}-").unwrap();
            for item in items {
                write_item(out, item, indent + 2, docs);
            }
        }
        node => writeln!(out, "{pad}- {}", inline(node)).unwrap(),
    }
}

/// Scalars, empty collections and lists of scalars fit on the key's line.
fn is_inline(node: &Node) -> bool {
    match node {
        Node::Scalar(_) => true,
        Node::Table(table) => table.entries.is_empty(),
        Node::Array(items) => items.iter().all(|i| matches!(i, Node::Scalar(_))),
    }
}

/// Flow style: `[a, b]`, `{ a: 1 }`.
fn inline(node: &Node) -> String {
    match node {
        Node::Scalar(s) => s.clone(),
        Node::Array(items) => {
            let items: Vec<_> = items.iter().map(inline).collect();
            format!("[{}]", items.join(", "))
        }
        Node::Table(table) if table.entries.is_empty() => "{}".to_string(),
        Node::Table(table) => {
            let entries: Vec<_> = table
                .entries
                .iter()
                .map(|e| format!("{}: {}", yaml_key(&e.key), inline(&e.value)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
    }
}

fn write_doc(out: &mut String, doc: &[String], indent: usize) {
    let pad = " ".repeat(indent);
    for line in doc {
        if line.is_empty() {
            writeln!(out, "{pad}#").unwrap();
        } else {
            writeln!(out, "{pad}# {line}").unwrap();
        }
    }
}
