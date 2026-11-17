//! The one table that says whether a client message reads or changes (FR-011, research R4), for
//! authorization (#1734). An exhaustive `match` with no wildcard: a new request kind does not
//! compile until it is classified here, and [`KINDS`] is checked against the schema.

use super::messages::{ClientRequest, RequestBody};

/// Whether a request only reads or changes something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Reads only.
    Read,
    /// Changes something (no kind does yet).
    Change,
}

/// Every request `type`, as on the wire.
pub const KINDS: &[&str] = &[
    "ping",
    "list_daemons",
    "list_sessions",
    "read_session",
    "subscribe",
];

/// Whether `request` reads or changes.
pub fn classify(request: &ClientRequest) -> Access {
    match request.body {
        RequestBody::Ping
        | RequestBody::ListDaemons
        | RequestBody::ListSessions
        | RequestBody::ReadSession { .. }
        | RequestBody::Subscribe { .. } => Access::Read,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use serde_json::Value;

    /// Every `type` constant the schema of `ClientRequest` names.
    fn schema_kinds() -> BTreeSet<String> {
        fn walk(value: &Value, out: &mut BTreeSet<String>) {
            match value {
                Value::Object(map) => {
                    if let Some(Value::Object(ty)) =
                        map.get("properties").and_then(|p| p.get("type"))
                    {
                        if let Some(Value::String(kind)) = ty.get("const") {
                            out.insert(kind.clone());
                        }
                        if let Some(Value::Array(kinds)) = ty.get("enum") {
                            out.extend(kinds.iter().filter_map(|k| k.as_str().map(str::to_owned)));
                        }
                    }
                    map.values().for_each(|v| walk(v, out));
                }
                Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
                _ => {}
            }
        }
        let schema = serde_json::to_value(schemars::schema_for!(ClientRequest)).unwrap();
        let mut out = BTreeSet::new();
        walk(&schema, &mut out);
        out
    }

    /// One request of `kind`, read from its JSON as a client sends it.
    fn sample(kind: &str) -> ClientRequest {
        let text = match kind {
            "read_session" => {
                r#"{"id":1,"type":"read_session","daemon":"d","project":"p","session":"s"}"#
                    .to_owned()
            }
            other => format!(r#"{{"id":1,"type":"{other}"}}"#),
        };
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{kind} does not parse: {e}"))
    }

    #[test]
    fn kinds_match_the_schema() {
        let kinds: BTreeSet<String> = KINDS.iter().map(|k| (*k).to_owned()).collect();
        assert_eq!(kinds, schema_kinds());
    }

    #[test]
    fn every_kind_is_classified_and_all_read_today() {
        for kind in KINDS {
            assert_eq!(classify(&sample(kind)), Access::Read, "{kind}");
        }
    }
}
