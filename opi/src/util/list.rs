//! [`List`]: a value written as `x` or `[x, y]`.

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{self, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// A value that can be written as `x` or `[x, y]`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn json<T: for<'de> Deserialize<'de>>(s: &str) -> T {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn one_or_many() {
        assert_eq!(json::<List<String>>(r#""x""#).as_slice(), ["x"]);
        assert_eq!(json::<List<String>>(r#"["x", "y"]"#).as_slice(), ["x", "y"]);
    }
}
