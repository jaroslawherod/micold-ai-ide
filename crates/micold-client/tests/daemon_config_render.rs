//! `render(&DaemonSettings, Kind)` writes one golden file per kind, and the file parses back to the
//! Settings it was rendered from (#1525, US1, FR-005).

#[path = "support/daemon_config_fixtures.rs"]
mod fixtures;

use fixtures::{golden, settings, KINDS};
use micold_client::daemon_config_render::render;
use micold_core::daemon_config::{parse, Listen, SocketSpec};

#[test]
fn each_kind_renders_byte_for_byte_to_its_golden_file() {
    for kind in KINDS {
        assert_eq!(render(&settings(kind), kind), golden(kind), "{kind:?}");
    }
}

#[test]
fn the_rendered_file_parses_back_to_its_settings() {
    for kind in KINDS {
        let s = settings(kind);
        let config = parse(&render(&s, kind), "daemon.toml")
            .unwrap_or_else(|e| panic!("{kind:?}: {}", e[0]));
        assert_eq!(config.daemon.name, s.name, "{kind:?}");
        assert_eq!(config.daemon.kind, kind);
        assert_eq!(config.daemon.image, s.image);
        assert_eq!(config.daemon.data_dir, s.data_dir);
        assert_eq!(config.daemon.idle_stop, s.idle_stop);
        assert_eq!(config.listen.and_then(|l| l.listen()), Some(s.listen));
        assert_eq!(config.projects.len(), s.projects.len());
    }
}

#[test]
fn a_golden_file_passes_validation() {
    for kind in KINDS {
        parse(&golden(kind), "daemon.toml").unwrap_or_else(|e| panic!("{kind:?}: {}", e[0]));
    }
}

#[test]
fn a_windows_path_is_a_literal_string_and_a_quote_is_escaped() {
    let mut s = settings(micold_core::daemon_config::Kind::Host);
    s.listen = Listen::Socket(SocketSpec::Auto);
    s.data_dir = Some(r"C:\Users\me\data".into());
    s.name = "A \"quoted\" name".into();
    let text = render(&s, micold_core::daemon_config::Kind::Host);
    assert!(text.contains(r"data_dir = 'C:\Users\me\data'"), "{text}");
    assert!(text.contains(r#"name = "A \"quoted\" name""#), "{text}");
}
