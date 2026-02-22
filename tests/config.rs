use std::path::PathBuf;
use udlna::cli::Args;
use udlna::config::{Config, FileConfig};

fn make_args(port: Option<u16>, name: Option<String>, paths: Vec<PathBuf>) -> Args {
    Args {
        port,
        name,
        paths,
        config: None,
        localhost: false,
    }
}

#[test]
fn test_defaults_when_nothing_set() {
    let args = make_args(None, None, vec![PathBuf::from("/tmp")]);
    let config = Config::resolve(None, &args);
    assert_eq!(config.port, 8200);
    assert!(
        config.name == "udlna" || config.name.starts_with("udlna@"),
        "expected default name to be 'udlna' or 'udlna@<hostname>', got: {}",
        config.name
    );
}

#[test]
fn test_cli_flag_overrides_default() {
    let args = make_args(Some(9000), None, vec![PathBuf::from("/tmp")]);
    let config = Config::resolve(None, &args);
    assert_eq!(config.port, 9000);
}

#[test]
fn test_toml_overrides_default() {
    let file = FileConfig { port: Some(7777), name: None, localhost: None };
    let args = make_args(None, None, vec![PathBuf::from("/tmp")]);
    let config = Config::resolve(Some(file), &args);
    assert_eq!(config.port, 7777);
}

#[test]
fn test_cli_overrides_toml() {
    let file = FileConfig { port: Some(7777), name: None, localhost: None };
    let args = make_args(Some(9000), None, vec![PathBuf::from("/tmp")]);
    let config = Config::resolve(Some(file), &args);
    assert_eq!(config.port, 9000); // CLI wins
}

#[test]
fn test_toml_parse() {
    let toml_str = "port = 9000\nname = \"Living Room\"\n";
    let parsed: FileConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(parsed.port, Some(9000));
    assert_eq!(parsed.name.as_deref(), Some("Living Room"));
}

#[test]
fn test_toml_unknown_fields_ignored() {
    // Future keys must not break parsing
    let toml_str = "port = 9000\nunknown_future_key = true\n";
    let parsed: Result<FileConfig, _> = toml::from_str(toml_str);
    assert!(parsed.is_ok());
}

#[test]
fn test_localhost_default_false() {
    let args = make_args(None, None, vec![PathBuf::from("/tmp")]);
    let config = Config::resolve(None, &args);
    assert!(!config.localhost, "localhost should default to false when neither CLI nor TOML sets it");
}
