use indexmap::IndexMap;

use crate::ast::{Component, Doc, Request, Response, Route};

use super::Rule;
use super::emit::Output;

/// Runs rules against one document. Each rule stages, filters and *collects*
/// its declarations; nothing is lowered to TS until [`Builder::render`], so a
/// rule can reference a type emitted by any other rule, in either order.
pub struct Builder<'a> {
    ir: &'a Doc,
    item_sets: ItemSets<'a>,
    output: Output<'a>,
    formats: IndexMap<String, String>,
}

impl<'a> Builder<'a> {
    pub fn new(ir: &'a Doc) -> Self {
        Self {
            ir,
            item_sets: ItemSets::new(),
            output: Output::default(),
            formats: IndexMap::new(),
        }
    }

    /// Schema `format` -> the TS type to use instead of the base type, e.g.
    /// `binary` -> `Blob`. None by default.
    pub fn formats(&mut self, formats: IndexMap<String, String>) -> &mut Self {
        self.formats = formats;
        self
    }

    pub fn with(&mut self, mut rule: Rule) -> anyhow::Result<&mut Self> {
        let sets = rule.run(self.ir);
        if let Some(emit) = &rule.emit {
            for stage in &sets.items {
                emit.collect(stage, &mut self.output)?;
            }
        }
        self.item_sets.extend(sets);
        Ok(self)
    }

    /// The TS module for every rule added so far.
    pub fn render(&self) -> anyhow::Result<String> {
        self.output.render(self.ir, &self.formats)
    }
}

#[derive(Debug, Clone)]
pub enum Stage<'a> {
    Routes(Vec<&'a Route>),
    Schemas(Vec<&'a Component>),
    Requests(Vec<(&'a Request, &'a Route)>),
    Responses(Vec<(&'a Response, &'a Route)>),
}

#[derive(Debug, Clone, Default)]
pub struct ItemSets<'a> {
    items: Vec<Stage<'a>>,
}
impl<'a> ItemSets<'a> {
    pub fn new() -> Self {
        Self { items: vec![] }
    }
    pub fn add(&mut self, stage: Stage<'a>) {
        self.items.push(stage);
    }
    pub fn extend(&mut self, other: ItemSets<'a>) {
        self.items.extend(other.items);
    }
    pub fn routes(&self) -> impl Iterator<Item = &'a Route> {
        self.items.iter().flat_map(|s| s.routes()).flatten()
    }
    pub fn schemas(&self) -> impl Iterator<Item = &'a Component> {
        self.items.iter().flat_map(|s| s.schemas()).flatten()
    }
    pub fn requests(&self) -> impl Iterator<Item = (&'a Request, &'a Route)> {
        self.items.iter().flat_map(|s| s.requests()).flatten()
    }
    pub fn responses(&self) -> impl Iterator<Item = (&'a Response, &'a Route)> {
        self.items.iter().flat_map(|s| s.responses()).flatten()
    }
}

macro_rules! impl_view_traits {
    ($([$variant:ident, $ty:ty, $get:ident, $from:ident, $update:ident, $complete:ident, $retain:ident]),*$(,)?) => {
        pub trait ViewRule<'a> {
            fn run(&mut self, _ir: &'a Doc) -> ItemSets<'a> {
                let mut item_sets = ItemSets::new();
                self.stage(&mut item_sets, _ir);
                for item in item_sets.items.iter_mut() {
                    self.retain(item);
                    self.update(item);
                    self.complete(item);
                }
                item_sets
            }
            fn stage(&mut self, _stage: &mut ItemSets<'a>, _ir: &'a Doc) {}
            fn retain(&mut self, view: &mut Stage<'a>) {
                match view { $( Stage::$variant(x) => x.retain(|item| self.$retain(item)), )* }
            }
            fn update(&mut self, view: &mut Stage<'a>) {
                match view {$( Stage::$variant(x) => x.into_iter().for_each(|item| self.$update(item)), )* }
            }
            fn complete(&mut self, stage: &mut Stage<'a>) {
                match stage {
                    $( Stage::$variant(x) => x.into_iter().for_each(|item| self.$complete(item)), )*
                }
            }
            $( fn $update(&mut self, _items: &mut $ty) { } )*
            $( fn $complete(&mut self, _items: &$ty) { } )*
            $( fn $retain(&mut self, _items: &$ty) -> bool { true } )*
        }

        impl<'a> Stage<'a> {
            $(
                pub fn $from(ir: &'a Doc) -> Self {
                    Self::$variant(ir.$get().into_iter().collect())
                }
                pub fn $get(&self) -> Option<impl Iterator<Item = $ty>> {
                    match self {
                        Self::$variant(items) => Some(items.iter().map(|r| *r)),
                        _ => None,
                    }
                }
            )*
        }
    };
}

impl_view_traits! {
    [Routes, &'a Route, routes, from_routes, update_route, complete_route, retain_route],
    [Schemas, &'a Component, schemas, from_schemas, update_schema, complete_schema, retain_schema],
    [Requests, (&'a Request, &'a Route), requests, from_requests, update_request, complete_request, retain_request],
    [Responses, (&'a Response, &'a Route), responses, from_responses, update_response, complete_response, retain_response],
}
