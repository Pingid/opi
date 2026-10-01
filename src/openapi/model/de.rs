//! Hand-written `Deserialize` impls for the model: the places where the
//! JSON shape doesn't map onto a struct (schema-or-bool, string-or-list,
//! `x-*` extension keys, integer map keys).

use std::fmt;

use indexmap::IndexMap;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::{Schema, SchemaOrBool};

impl<'de> Deserialize<'de> for SchemaOrBool {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = SchemaOrBool;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a schema object or boolean")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(SchemaOrBool::Bool(v))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let schema = Schema::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(SchemaOrBool::Schema(Box::new(schema)))
            }

            /// Draft-4 tuple `items: [A, B]`. No IR equivalent; accept as
            /// "anything" rather than failing the whole document.
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(SchemaOrBool::Bool(true))
            }
        }
        deserializer.deserialize_any(V)
    }
}
/// A present key's value, `null` included.
pub(super) fn present<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// `required: [..]` on an object schema. Some specs also put the 2.0-style
/// `required: true` on a property schema; that's ignored, not an error.
pub(super) fn names_or_nothing<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Array(names) => names
            .into_iter()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    })
}

/// A map minus its `x-*` extension keys, whose values can be anything.
/// Keys may be integers (YAML `200:` in `responses`).
pub(super) fn without_extensions<'de, D, T>(
    deserializer: D,
) -> Result<IndexMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct V<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
        type Value = IndexMap<String, T>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = IndexMap::new();
            while let Some(Key(key)) = map.next_key()? {
                if key.starts_with("x-") {
                    map.next_value::<IgnoredAny>()?;
                } else {
                    let value = map.next_value()?;
                    out.insert(key, value);
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(V(std::marker::PhantomData))
}

/// A map key written as a string or an integer.
struct Key(String);

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Key;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or integer key")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Key(v.to_string()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Key(v.to_string()))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Key(v.to_string()))
            }
        }
        deserializer.deserialize_any(V)
    }
}
