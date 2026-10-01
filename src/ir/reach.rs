//! Which named schemas are reachable from a set of operations, following
//! `Ref`s transitively. Used to drop schemas nothing generated refers to.

use std::collections::HashSet;

use super::{Api, Operation, Schema, SchemaKind};

impl Schema {
    /// Call `f` with the name of every schema this one references directly
    /// (not following the refs themselves).
    pub fn visit_refs<'s>(&'s self, f: &mut impl FnMut(&'s str)) {
        if let SchemaKind::Ref { name } = &self.kind {
            f(name);
        }
        for child in self.children() {
            child.visit_refs(f);
        }
    }
}

impl Operation {
    /// Every schema in the params, request bodies and responses.
    pub fn schemas(&self) -> impl Iterator<Item = &Schema> {
        let requests = self.requests.iter().filter_map(|r| r.body.as_ref());
        let responses = self.responses.iter().filter_map(|r| r.body.as_ref());
        let params = self.params.iter().map(|p| &p.schema);
        params.chain(requests).chain(responses)
    }

    /// Every schema the operation's params, body and responses reference.
    pub fn visit_refs<'s>(&'s self, f: &mut impl FnMut(&'s str)) {
        for schema in self.schemas() {
            schema.visit_refs(f);
        }
    }
}

impl Api {
    /// Names of the schemas reachable from `ops` or from any schema named in
    /// `roots`, in spec order. Cycles are fine; unknown names are ignored.
    pub fn reachable_schemas<'a>(
        &'a self,
        ops: &[&'a Operation],
        roots: impl IntoIterator<Item = &'a str>,
    ) -> Vec<&'a str> {
        let mut todo: Vec<&str> = roots.into_iter().collect();
        for op in ops {
            op.visit_refs(&mut |name| todo.push(name));
        }

        let mut seen = HashSet::new();
        while let Some(name) = todo.pop() {
            if !seen.insert(name) {
                continue;
            }
            if let Some(schema) = self.schemas.get(name) {
                schema.visit_refs(&mut |next| todo.push(next));
            }
        }

        self.schemas
            .keys()
            .map(String::as_str)
            .filter(|name| seen.contains(name))
            .collect()
    }
}
