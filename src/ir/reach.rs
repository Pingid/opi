//! Which named schemas are reachable from a set of operations, following
//! `Ref`s transitively. Used to drop schemas nothing generated refers to.

use std::collections::HashSet;

use super::{Api, Operation, Schema, SchemaKind};

impl Schema {
    /// Call `f` with the name of every schema this one references directly
    /// (not following the refs themselves).
    pub fn visit_refs<'s>(&'s self, f: &mut impl FnMut(&'s str)) {
        match &self.kind {
            SchemaKind::Ref(name) => f(name),
            SchemaKind::Array(items) => items.visit_refs(f),
            SchemaKind::Object(object) => {
                for property in object.properties.values() {
                    property.schema.visit_refs(f);
                }
                if let Some(additional) = &object.additional {
                    additional.visit_refs(f);
                }
            }
            SchemaKind::Union(members) | SchemaKind::Intersection(members) => {
                for member in members {
                    member.visit_refs(f);
                }
            }
            SchemaKind::Unknown
            | SchemaKind::Never
            | SchemaKind::String { .. }
            | SchemaKind::Number { .. }
            | SchemaKind::Integer { .. }
            | SchemaKind::Boolean
            | SchemaKind::Null
            | SchemaKind::Enum(_) => {}
        }
    }
}

impl Operation {
    /// Every schema the operation's params, body and responses reference.
    pub fn visit_refs<'s>(&'s self, f: &mut impl FnMut(&'s str)) {
        let params = &self.params;
        let all = params.path.iter().chain(&params.query).chain(&params.querystring);
        for param in all.chain(&params.header).chain(&params.cookie) {
            param.schema.visit_refs(f);
        }
        for content in self.body.iter().flat_map(|b| b.content.values()) {
            content.schema.visit_refs(f);
        }
        for content in self.responses.iter().flat_map(|r| r.content.values()) {
            content.schema.visit_refs(f);
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
