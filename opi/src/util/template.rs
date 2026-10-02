use schemars::JsonSchema;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Template {
    pub variables: Vec<String>,
    pub literals: Vec<String>, // Always holds variables.len() + 1 entries
}

impl Template {
    pub const EMPTY: Template = Template {
        variables: Vec::new(),
        literals: Vec::new(),
    };

    pub fn variables(&self) -> impl Iterator<Item = &String> {
        self.variables.iter()
    }

    pub fn has_variables(&self) -> bool {
        !self.variables.is_empty()
    }

    pub fn parse(input: &str) -> Result<Self, ParseTemplateError> {
        let mut variables = Vec::new();
        let mut literals = Vec::new();
        let mut literal_buf = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    // Check for escaped "{{"
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        literal_buf.push('{');
                    } else {
                        // Flush the accumulated literal part (even if empty)
                        literals.push(std::mem::take(&mut literal_buf));

                        // Read until matching '}'
                        let mut var_buf = String::new();
                        let mut closed = false;
                        for next_ch in chars.by_ref() {
                            if next_ch == '}' {
                                closed = true;
                                break;
                            }
                            var_buf.push(next_ch);
                        }

                        if !closed {
                            return Err(ParseTemplateError::UnclosedBrace);
                        }

                        let trimmed = var_buf.trim();
                        if trimmed.is_empty() {
                            return Err(ParseTemplateError::EmptyVariable);
                        }

                        variables.push(trimmed.to_string());
                    }
                }
                '}' => {
                    // Check for escaped "}}"
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        literal_buf.push('}');
                    } else {
                        return Err(ParseTemplateError::UnmatchedCloseBrace);
                    }
                }
                other => {
                    literal_buf.push(other);
                }
            }
        }

        // Push the trailing literal (even if empty)
        literals.push(literal_buf);

        Ok(Template {
            variables,
            literals,
        })
    }
}

// ---------------------------------------------------------------------------
// Error Handling & Standard Traits
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseTemplateError {
    UnclosedBrace,
    UnmatchedCloseBrace,
    EmptyVariable,
}

impl fmt::Display for ParseTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnclosedBrace => write!(f, "unclosed '{{' in template expression"),
            Self::UnmatchedCloseBrace => write!(f, "unmatched '}}' in template expression"),
            Self::EmptyVariable => write!(f, "empty variable expression '{{}}'"),
        }
    }
}

impl std::error::Error for ParseTemplateError {}

impl FromStr for Template {
    type Err = ParseTemplateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Template::parse(s)
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.literals.is_empty() {
            return Ok(());
        }

        // Helper to escape braces in literal chunks when serializing back
        fn write_escaped(f: &mut fmt::Formatter<'_>, lit: &str) -> fmt::Result {
            for ch in lit.chars() {
                match ch {
                    '{' => write!(f, "{{{{")?,
                    '}' => write!(f, "}}}}")?,
                    c => write!(f, "{c}")?,
                }
            }
            Ok(())
        }

        write_escaped(f, &self.literals[0])?;
        for (i, var) in self.variables.iter().enumerate() {
            write!(f, "{{{var}}}")?;
            write_escaped(f, &self.literals[i + 1])?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Serde Deserialization & Serialization
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for Template {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TemplateExprVisitor;

        impl<'de> Visitor<'de> for TemplateExprVisitor {
            type Value = Template;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string template expression")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Template::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(TemplateExprVisitor)
    }
}

impl Serialize for Template {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Serialised as a string, so that's its schema too.
impl JsonSchema for Template {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Template".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "Literal text with `{variable}` placeholders; `{{` and `}}` are literal braces."
        })
    }
}
