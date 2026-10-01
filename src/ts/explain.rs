//! The `--explain` report.

use std::fmt::{self, Write};

use indexmap::IndexMap;

use super::Generator;
use super::emit::EmitReport;
use crate::config::SchemaEmit;

/// What the config selected: a line per stage, with `⚠` warnings under it.
pub(super) struct Report {
    lines: Vec<Line>,
}

struct Line {
    label: String,
    text: String,
    warnings: Vec<String>,
}

impl Report {
    /// Run [`Generator::statements`] first: `emit` fills in its reports.
    pub fn new(g: &Generator) -> Self {
        let mut lines = vec![filter_line(g), schemas_line(g)];
        lines.extend(g.sel.reports.borrow().iter().map(emit_line));
        Self { lines }
    }
}

impl Line {
    fn new(label: &str, text: String) -> Self {
        Self {
            label: label.to_string(),
            text,
            warnings: Vec::new(),
        }
    }
}

/// `179 operations → 150 (29 excluded: deprecated)`.
fn filter_line(g: &Generator) -> Line {
    let total = g.api.operations.len();
    let mut text = format!("{total} operations");
    if !g.sel.excluded.is_empty() {
        let kept = g.sel.ops.len();
        let why = counts(&g.sel.excluded);
        write!(text, " → {kept} ({} excluded: {why})", total - kept).unwrap();
    }
    Line::new("filter", text)
}

/// `12 → 7 (referenced)`.
fn schemas_line(g: &Generator) -> Line {
    let mode = match g.config.schemas.emit {
        SchemaEmit::All => "all",
        SchemaEmit::Referenced if g.config.schemas.keep.is_some() => "referenced + keep",
        SchemaEmit::Referenced => "referenced",
    };
    let text = format!("{} → {} ({mode})", g.api.schemas.len(), g.sel.schemas.len());
    Line::new("schemas", text)
}

/// `3 types, largest "ShopRoutes" (2) (1 dropped: deprecated)`, warning
/// per variable that operations were skipped for lacking.
fn emit_line(report: &EmitReport) -> Line {
    let mut text = summary(report);
    if !report.dropped.is_empty() {
        let dropped: usize = report.dropped.values().sum();
        write!(text, " ({dropped} dropped: {})", counts(&report.dropped)).unwrap();
    }
    let warnings = report
        .skipped
        .iter()
        .map(|(var, ops)| format!("{} no {{{var}}} → skipped", have(ops.len())))
        .collect();
    Line {
        label: format!("emit {:?}", report.label),
        text,
        warnings,
    }
}

/// `2 entries` for a single type, else `3 types, largest "X" (2)`.
fn summary(report: &EmitReport) -> String {
    match (report.grouped, report.types.as_slice()) {
        (false, [(_, entries)]) => plural(*entries, "entry", "entries"),
        (_, types) => {
            let mut text = plural(types.len(), "type", "types");
            // The last of equally large types, as `max_by_key` picks.
            if let Some((name, entries)) = types.iter().max_by_key(|(_, n)| *n) {
                write!(text, ", largest {name:?} ({entries})").unwrap();
            }
            text
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let labels = self.lines.iter().map(|l| l.label.chars().count());
        let width = labels.max().unwrap_or(0) + 2;
        for line in &self.lines {
            writeln!(f, "{:<width$}{}", line.label, line.text)?;
            for warning in &line.warnings {
                writeln!(f, "  ⚠ {warning}")?;
            }
        }
        Ok(())
    }
}

/// `179 deprecated`, or `deprecated` when it's the only reason.
fn counts(by_field: &IndexMap<String, usize>) -> String {
    match by_field.iter().collect::<Vec<_>>().as_slice() {
        [(field, _)] => field.to_string(),
        all => all
            .iter()
            .map(|(field, n)| format!("{n} {field}"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `1 operation has` / `3 operations have`.
fn have(n: usize) -> String {
    if n == 1 {
        "1 operation has".to_string()
    } else {
        format!("{n} operations have")
    }
}
