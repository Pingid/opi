//! A response status: `200`, `2XX` or `default`.

use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Serialize, Serializer};

/// Serialised (and reflected) as written in a spec: `"200"`, `"2XX"`,
/// `"default"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "facet", derive(facet::Facet), facet(proxy = String))]
#[repr(u8)]
pub enum Status {
    /// `200`
    Code(u16),
    /// `2XX`, stored as the leading digit.
    Range(u16),
    /// `default`
    Default,
}

impl Status {
    /// Whether `other` is one of the statuses `self` stands for: a range
    /// covers its codes and the spec's own `2XX`; a code or `default` only
    /// itself.
    pub fn covers(self, other: Status) -> bool {
        match (self, other) {
            (Status::Range(range), Status::Code(code)) => code / 100 == range,
            (a, b) => a == b,
        }
    }
}

/// `200`, `2xx` / `2XX`, or `default` (case-insensitive).
impl FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.eq_ignore_ascii_case("default") {
            return Ok(Status::Default);
        }
        if let [class @ b'1'..=b'5', x, y] = s.as_bytes()
            && x.eq_ignore_ascii_case(&b'X')
            && y.eq_ignore_ascii_case(&b'X')
        {
            return Ok(Status::Range(u16::from(class - b'0')));
        }
        if let Ok(code) = s.parse() {
            return Ok(Status::Code(code));
        }
        bail!("unknown status {s:?} (expected e.g. 200, 2xx or default)")
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Code(c) => write!(f, "{c}"),
            Status::Range(r) => write!(f, "{r}XX"),
            Status::Default => f.write_str("default"),
        }
    }
}

impl Serialize for Status {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Conversions for facet's `proxy = String`.
impl TryFrom<String> for Status {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        s.parse()
    }
}

impl From<&Status> for String {
    fn from(status: &Status) -> Self {
        status.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_display() {
        let parse = |s: &str| s.parse::<Status>();
        assert_eq!(parse("200").unwrap(), Status::Code(200));
        assert_eq!(parse("2xx").unwrap(), Status::Range(2));
        assert_eq!(parse("2xX").unwrap(), Status::Range(2));
        assert_eq!(parse("DEFAULT").unwrap(), Status::Default);
        for s in ["200", "2XX", "default"] {
            assert_eq!(parse(s).unwrap().to_string(), s);
        }
        for s in ["2xxx", "02xx", "6xx", "abc", ""] {
            let err = parse(s).unwrap_err().to_string();
            assert!(err.contains("unknown status"), "{s:?}: {err}");
        }
    }

    #[test]
    fn covers() {
        assert!(Status::Code(200).covers(Status::Code(200)));
        assert!(!Status::Code(200).covers(Status::Code(201)));
        assert!(!Status::Code(200).covers(Status::Range(2)));
        assert!(Status::Range(2).covers(Status::Code(204)));
        assert!(Status::Range(2).covers(Status::Range(2)));
        assert!(!Status::Range(2).covers(Status::Code(404)));
        assert!(Status::Default.covers(Status::Default));
        assert!(!Status::Default.covers(Status::Code(200)));
    }
}
