//! Architecture guard: the Rust side is a library, never a CLI.
//!
//! Panduck ships two thin bindings (`panduck-napi` and `panduck-wasm`) and no
//! binary entry point. Argument parsing, usage text, exit codes and user-facing
//! wording belong to the TypeScript CLI. This test fails the build if that
//! boundary erodes.

use std::fs;
use std::path::{Path, PathBuf};

/// Returns the repository root (`panduck/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root must resolve")
}

/// Collects files with the given extension below `dir`.
fn collect_files(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display())) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_files(&path, extension, out);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            out.push(path);
        }
    }
}

/// Strips line comments so documentation may mention CLI concerns without
/// owning them.
fn code_of(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

#[test]
fn rust_side_declares_no_binary_target() {
    let root = repo_root();
    let mut manifests = Vec::new();
    collect_files(&root, "toml", &mut manifests);

    let mut offenders = Vec::new();
    for manifest in manifests {
        if manifest.file_name().and_then(|value| value.to_str()) != Some("Cargo.toml") {
            continue;
        }
        let text = fs::read_to_string(&manifest).expect("manifest must be readable");
        if text.contains("[[bin]]") {
            offenders.push(manifest.display().to_string());
        }
    }

    let mut sources = Vec::new();
    collect_files(&root.join("projects"), "rs", &mut sources);
    for source in sources {
        if source.ends_with(Path::new("src").join("main.rs")) {
            offenders.push(source.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "the Rust workspace must stay a library: {offenders:?}"
    );
}

#[test]
fn library_sources_hold_no_cli_concepts() {
    const FORBIDDEN: &[&str] = &[
        "std::env::args",
        "std::process::exit",
        "exit_code",
        "ExitCode",
        "usage:",
        "--from",
        "--to",
        "--output",
        "--diagnostics",
    ];

    let root = repo_root().join("projects").join("panduck.rs");
    let mut sources = Vec::new();
    collect_files(&root, "rs", &mut sources);

    let mut offenders = Vec::new();
    for source in sources {
        if source
            .components()
            .any(|component| component.as_os_str() == "tests")
        {
            continue;
        }
        let text = fs::read_to_string(&source).expect("source must be readable");
        for (index, line) in text.lines().enumerate() {
            let code = code_of(line);
            for pattern in FORBIDDEN {
                if code.contains(pattern) {
                    offenders.push(format!("{}:{}: {pattern}", source.display(), index + 1));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "library sources must not own CLI concepts: {offenders:?}"
    );
}
