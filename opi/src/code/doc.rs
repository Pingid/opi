//! Doc comments.
//!
//! oxc's codegen prints comments by looking them up by the `span.start` of
//! the node they're attached to, and slicing their text out of the program's
//! source text. Since generated nodes have no source, we fake both: every
//! documented node gets a unique synthetic span start, and the comment text
//! is appended to a buffer that becomes the program's `source_text` at
//! codegen time.

use std::cell::{Cell, RefCell};

use oxc_ast::ast::*;
use oxc_span::{GetSpanMut, Span};

use super::TypeBuilder;

#[derive(Default)]
pub(super) struct Docs {
    pub(super) source: RefCell<String>,
    pub(super) comments: RefCell<Vec<Comment>>,
    /// Offset from [`ANCHOR_BASE`] of the next anchor.
    next_anchor: Cell<u32>,
}

/// Synthetic anchors live far above any real offset, so they can't collide
/// with span 0 (every built node).
const ANCHOR_BASE: u32 = 1 << 31;

impl<'a> TypeBuilder<'a> {
    /// Attach a `/** ... */` comment to a node. `lines` are the raw lines of
    /// the comment body (e.g. `["@description The component"]`).
    pub fn with_doc<N: GetSpanMut>(&self, mut node: N, lines: &[String]) -> N {
        if lines.is_empty() {
            return node;
        }
        let offset = self.docs.next_anchor.get();
        self.docs.next_anchor.set(offset + 1);
        let anchor = ANCHOR_BASE + offset;
        *node.span_mut() = Span::new(anchor, anchor);

        let text = jsdoc_text(lines);
        let mut source = self.docs.source.borrow_mut();
        let start = source.len() as u32;
        source.push_str(&text);
        let end = source.len() as u32;

        let mut comment = Comment::new(
            start,
            end,
            if lines.len() > 1 {
                CommentKind::MultiLineBlock
            } else {
                CommentKind::SingleLineBlock
            },
        );
        comment.attached_to = anchor;
        comment.position = CommentPosition::Leading;
        comment.content = CommentContent::Jsdoc;
        comment.newlines = CommentNewlines::Leading | CommentNewlines::Trailing;
        self.docs.comments.borrow_mut().push(comment);
        node
    }
}

fn jsdoc_text(lines: &[String]) -> String {
    let escape = |l: &String| l.replace("*/", "*\\/");
    match lines {
        [line] => format!("/** {} */", escape(line)),
        lines => {
            let mut out = String::from("/**\n");
            for line in lines {
                // No trailing space on a blank line.
                out.push_str(if line.is_empty() { " *" } else { " * " });
                out.push_str(&escape(line));
                out.push('\n');
            }
            out.push_str(" */");
            out
        }
    }
}
