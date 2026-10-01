//! Rust doc comments as text: rustdoc's leading space dropped, intra-doc
//! links (`` [`Foo`] ``) as plain code spans.

pub(super) fn clean_line(line: &str) -> String {
    let line = line.strip_prefix(' ').unwrap_or(line).trim_end();
    line.replace("[`", "`").replace("`]", "`")
}

/// Comment lines: code fences are dropped (the example inside stays).
pub(super) fn clean_doc(doc: &[&str]) -> Vec<String> {
    doc.iter()
        .map(|l| clean_line(l))
        .filter(|l| !l.starts_with("```"))
        .collect()
}

/// One description, trimmed; `None` when there's nothing to say.
pub(super) fn doc_text(lines: &[&str]) -> Option<String> {
    let text = lines
        .iter()
        .map(|l| clean_line(l))
        .collect::<Vec<_>>()
        .join("\n");
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
