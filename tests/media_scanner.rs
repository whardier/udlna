use std::path::PathBuf;
use udlna::media::scanner::scan;

#[test]
fn scan_nonexistent_path_returns_empty_library() {
    let paths = vec![PathBuf::from("/nonexistent/path/does/not/exist")];
    let library = scan(&paths);
    assert_eq!(library.items.len(), 0);
}

#[test]
fn scan_empty_paths_returns_empty_library() {
    let paths: Vec<PathBuf> = vec![];
    let library = scan(&paths);
    assert_eq!(library.items.len(), 0);
}
