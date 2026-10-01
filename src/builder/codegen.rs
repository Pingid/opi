//! Printing: statements to source, and one type to a line.

use oxc_allocator::ArenaVec;
use oxc_ast::ast::*;
use oxc_codegen::Codegen;
use oxc_span::SPAN;

use super::TypeBuilder;

impl<'a> TypeBuilder<'a> {
    pub fn code_gen<I: Into<Statement<'a>>>(&self, body: impl IntoIterator<Item = I>) -> String {
        let source_text = self.str(&self.docs.source.borrow());
        let comments = self.docs.comments.borrow();
        let program = Program::new(
            SPAN,
            SourceType::ts(),
            source_text,
            ArenaVec::from_iter_in(comments.iter().copied(), &self.ast),
            None,
            ArenaVec::new_in(&self.ast),
            ArenaVec::from_iter_in(body.into_iter().map(|i| i.into()), &self.ast),
            &self.ast,
        );

        Codegen::new().build(&program).code
    }

    /// One type printed on its own, on one line: `{ id: string; tags?: string[] }`.
    /// For types without docs (which would be folded into the line too).
    pub fn type_to_string(&self, type_: TSType<'a>) -> String {
        const PREFIX: &str = "type __T = ";
        let code = self.code_gen([Statement::from(self.type_alias("__T", type_))]);
        let code = code.trim_end();
        let code = code.strip_prefix(PREFIX).unwrap_or(code);
        let code = code.strip_suffix(';').unwrap_or(code);
        // Codegen only breaks lines between members (string literals are
        // escaped), so folding them is safe.
        let mut out = String::with_capacity(code.len());
        for line in code.lines() {
            let line = line.trim_start();
            if !out.is_empty() {
                if line.starts_with('}') && out.ends_with(';') {
                    out.pop();
                }
                out.push(' ');
            }
            out.push_str(line);
        }
        out
    }
}
