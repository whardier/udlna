use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let version = git_version()
        .unwrap_or_else(|| {
            let v = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
            if v.is_empty() { "unknown".to_string() } else { v }
        });

    println!("cargo:rustc-env=GIT_VERSION={version}");
}

fn git_version() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--always", "--long", "--dirty", "--tags", "--match", "v[0-9]*"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    let raw = raw.trim();

    if raw.is_empty() {
        return None;
    }

    // Tag-based: v1.2.3-N-gHASH or v1.2.3-N-gHASH-dirty — strip the leading 'v'
    if raw.starts_with('v') {
        return Some(raw.trim_start_matches('v').to_string());
    }

    // No tags: just a hash (possibly with -dirty suffix)
    Some(format!("0.0.0-g{raw}"))
}
