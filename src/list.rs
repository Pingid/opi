//! [`List`]: a value written as `x` or `[x, y]`; and [`distinct`].

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{self, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// A value that can be written as `x` or `[x, y]`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(untagged))]
#[repr(u8)]
pub enum List<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> List<T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            List::One(v) => std::slice::from_ref(v),
            List::Many(v) => v,
        }
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        List::Many(Vec::new())
    }
}

/// `"x"` and `["x"]` are the same list.
impl<T: PartialEq> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T> From<Vec<T>> for List<T> {
    fn from(v: Vec<T>) -> Self {
        List::Many(v)
    }
}

/// Read directly rather than as an untagged enum, which buffers every value
/// (slow on big specs) and reports "didn't match any variant" instead of the
/// real error: a sequence is `Many`, anything else is one `T`.
impl<'de, T: Deserialize<'de>> Deserialize<'de> for List<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ListVisitor(PhantomData))
    }
}

struct ListVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for ListVisitor<T> {
    type Value = List<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a value or a list of them")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Vec::deserialize(SeqAccessDeserializer::new(seq)).map(List::Many)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        T::deserialize(MapAccessDeserializer::new(map)).map(List::One)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(List::One)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(List::One)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(List::One)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(List::One)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(List::One)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        T::deserialize(v.into_deserializer()).map(List::One)
    }
}

/// `items` without repeats, in first-seen order.
pub fn distinct<T: PartialEq>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut out = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::StatusPattern;

    fn yaml<T: for<'de> Deserialize<'de>>(s: &str) -> T {
        serde_yaml::from_str(s).unwrap()
    }

    fn json<T: for<'de> Deserialize<'de>>(s: &str) -> T {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn one_or_many() {
        assert_eq!(yaml::<List<String>>("x"), List::One("x".into()));
        assert_eq!(yaml::<List<String>>("x"), List::Many(vec!["x".into()]));
        assert_eq!(yaml::<List<String>>("[x, y]").as_slice(), ["x", "y"]);
        assert_eq!(json::<List<String>>(r#""x""#).as_slice(), ["x"]);
        assert_eq!(json::<List<String>>(r#"["x", "y"]"#).as_slice(), ["x", "y"]);
        assert_eq!(
            yaml::<List<StatusPattern>>("200").as_slice(),
            [StatusPattern("200".into())]
        );
        assert_eq!(
            yaml::<List<StatusPattern>>("[200, 2xx, default]")
                .as_slice()
                .len(),
            3
        );
        assert_eq!(
            json::<List<StatusPattern>>(r#"[200, "2xx"]"#)
                .as_slice()
                .len(),
            2
        );
        let err = serde_yaml::from_str::<List<String>>("200")
            .unwrap_err()
            .to_string();
        assert!(err.contains("expected a string"), "{err}");
    }

    #[test]
    fn distinct_keeps_first_seen_order() {
        assert_eq!(distinct(["b", "a", "b", "c", "a"]), ["b", "a", "c"]);
    }
}
