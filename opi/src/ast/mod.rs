mod schema;
pub use schema::*;

mod route;
pub use route::*;

#[derive(Debug)]
pub struct Doc {
    pub routes: Vec<Route>,
    pub schemas: Vec<Component>,
}

/// A named schema from `components/schemas`. The name is the raw key, used
/// for the `{name}` variable and for resolving `Ref`s.
#[derive(Debug)]
pub struct Component {
    pub name: String,
    pub schema: Schema,
}

impl Doc {
    pub fn routes(&self) -> impl Iterator<Item = &Route> {
        self.routes.iter()
    }
    pub fn schemas(&self) -> impl Iterator<Item = &Component> {
        self.schemas.iter()
    }
    /// The component schema a `Ref(name)` points at.
    pub fn schema(&self, name: &str) -> Option<&Schema> {
        self.schemas
            .iter()
            .find(|c| c.name == name)
            .map(|c| &c.schema)
    }
    pub fn requests(&self) -> impl Iterator<Item = (&Request, &Route)> {
        self.routes
            .iter()
            .flat_map(|route| route.request.iter().map(move |req| (req, route)))
    }
    pub fn responses(&self) -> impl Iterator<Item = (&Response, &Route)> {
        self.routes
            .iter()
            .flat_map(|route| route.response.iter().map(move |res| (res, route)))
    }
}
