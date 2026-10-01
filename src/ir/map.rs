//! The same operation over another schema representation: what `--emit-ir`
//! uses to replace every [`Schema`](super::Schema) with its TS source.

use super::{Operation, Param, Params, Request, Response};

impl<S> Operation<S> {
    /// The operation with every schema mapped through `f`.
    pub fn map<T>(&self, mut f: impl FnMut(&S) -> T) -> Operation<T> {
        Operation {
            id: self.id.clone(),
            method: self.method.clone(),
            path: self.path.clone(),
            summary: self.summary.clone(),
            description: self.description.clone(),
            deprecated: self.deprecated,
            tags: self.tags.clone(),
            params: self.params.map(&mut f),
            requests: self.requests.iter().map(|r| r.map(&mut f)).collect(),
            responses: self.responses.iter().map(|r| r.map(&mut f)).collect(),
        }
    }
}

impl<S> Params<S> {
    pub fn map<T>(&self, f: &mut impl FnMut(&S) -> T) -> Params<T> {
        Params {
            path: self.path.iter().map(|p| p.map(f)).collect(),
            query: self.query.iter().map(|p| p.map(f)).collect(),
            querystring: self.querystring.as_ref().map(|p| p.map(f)),
            header: self.header.iter().map(|p| p.map(f)).collect(),
            cookie: self.cookie.iter().map(|p| p.map(f)).collect(),
        }
    }
}

impl<S> Param<S> {
    pub fn map<T>(&self, f: &mut impl FnMut(&S) -> T) -> Param<T> {
        Param {
            name: self.name.clone(),
            required: self.required,
            description: self.description.clone(),
            deprecated: self.deprecated,
            schema: f(&self.schema),
        }
    }
}

impl<S> Request<S> {
    pub fn map<T>(&self, f: &mut impl FnMut(&S) -> T) -> Request<T> {
        Request {
            content_type: self.content_type.clone(),
            body: self.body.as_ref().map(f),
            stream: self.stream,
            description: self.description.clone(),
        }
    }
}

impl<S> Response<S> {
    pub fn map<T>(&self, f: &mut impl FnMut(&S) -> T) -> Response<T> {
        Response {
            status: self.status,
            content_type: self.content_type.clone(),
            body: self.body.as_ref().map(f),
            stream: self.stream,
            description: self.description.clone(),
        }
    }
}
