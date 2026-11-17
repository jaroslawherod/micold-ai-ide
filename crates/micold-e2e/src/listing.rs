//! The orchestrator's listing as the `e2e-link list` verb prints it: `{ "daemons": [{ "id",
//! "name", "health", .. }], .. }` (`micold-daemon`'s `orchestrator::query::listing`).

use serde::Deserialize;

/// One daemon of the listing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Daemon {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// `online`, `offline` or `needs-update`.
    pub health: String,
}

#[derive(Deserialize)]
struct Listing {
    daemons: Vec<Daemon>,
}

/// The daemons of the listing `json`; an error naming what was not a listing.
pub fn parse_listing(json: &str) -> Result<Vec<Daemon>, String> {
    serde_json::from_str::<Listing>(json.trim())
        .map(|l| l.daemons)
        .map_err(|e| format!("not a listing ({e}): {json:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_id_name_and_health_and_ignores_the_rest() {
        let json = r#"{"daemons":[{"id":"a","name":"A","health":"offline","last_seen":1,"kind":"kubernetes"}],"orchestrator_version":83}"#;
        assert_eq!(
            parse_listing(json),
            Ok(vec![Daemon {
                id: "a".into(),
                name: "A".into(),
                health: "offline".into()
            }])
        );
    }

    #[test]
    fn an_empty_listing_has_no_daemons() {
        assert_eq!(
            parse_listing("{\"daemons\":[],\"orchestrator_version\":83}\n"),
            Ok(vec![])
        );
    }

    #[test]
    fn anything_else_is_an_error_showing_it() {
        let err = parse_listing("error: no pod").unwrap_err();
        assert!(err.contains("error: no pod"), "{err}");
    }
}
