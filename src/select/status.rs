//! `status` patterns: `200`, `2xx`, `default`; parsed as an [`ir::Status`](Status)
//! and matched with [`Status::covers`].

use std::fmt;

use anyhow::Result;

use serde::{Deserialize, Serialize};

use crate::ir::Status;

/// A response status to match: `200`, `2xx` / `2XX` (any 2xx code, and a
/// spec's own `2XX` range) or `default`. Read from a number or a string and
/// checked by [`Where::validate`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(transparent))]
pub struct StatusPattern(pub String);
impl StatusPattern {
    pub(super) fn parse(&self) -> Result<Status> {
        self.0.trim().parse()
    }

    pub fn matches(&self, status: Status) -> bool {
        self.parse().is_ok_and(|pattern| pattern.covers(status))
    }
}

impl Serialize for StatusPattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.parse::<u64>() {
            Ok(n) => serializer.serialize_u64(n),
            Err(_) => serializer.serialize_str(&self.0),
        }
    }
}

impl<'de> Deserialize<'de> for StatusPattern {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = StatusPattern;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a status: 200, \"2xx\" or \"default\"")
            }
            fn visit_u64<E>(self, v: u64) -> Result<StatusPattern, E> {
                Ok(StatusPattern(v.to_string()))
            }
            fn visit_i64<E>(self, v: i64) -> Result<StatusPattern, E> {
                Ok(StatusPattern(v.to_string()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<StatusPattern, E> {
                if v.fract() == 0.0 && v >= 0.0 {
                    Ok(StatusPattern((v as u64).to_string()))
                } else {
                    Err(E::custom(format!("unknown status {v}")))
                }
            }
            fn visit_str<E>(self, v: &str) -> Result<StatusPattern, E> {
                Ok(StatusPattern(v.to_string()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
