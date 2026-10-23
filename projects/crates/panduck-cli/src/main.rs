//! Panduck CLI — thin command-line wrapper over `panduck-convert` library.
//!
//! Commands directly call the published conversion functions. No new format
//! semantics, scheduling layers, or NAPI round-trips introduced here.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use panduck_convert::{
    convert_file, convert_markdown_project_file, inspect_docx_index, supports_markdown_project_source, supports_route,
};

/// Panduck — document conversion toolkit CLI.
#[derive(Debug, Parser)]
#[command(name = "panduck", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show Panduck version and wired reader / writer formats.
    Version,

    /// Convert a file from one format to another.
    Convert {
        /// Source format (`docx`, `doc`, `pdf`, `epub`, `html`, `markdown`, `notedown`).
        #[arg(long = "from", short = 'f')]
        from: String,
        /// Target format (`markdown`, `html`, `docx`, `pdf`).
        #[arg(long = "to", short = 't')]
        to: String,
        /// Input file path.
        input: PathBuf,
        /// Output file path (derived from input + target if omitted).
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// Convert a source file into a Markdown project directory (index.md + assets/).
    Project {
        /// Source format.
        #[arg(long = "from", short = 'f')]
        from: String,
        /// Input file path.
        input: PathBuf,
        /// Output directory (created if missing).
        output_dir: PathBuf,
    },

    /// Inspect internal structure of a source without converting.
    Inspect {
        #[command(subcommand)]
        kind: InspectKind,
    },
}

#[derive(Debug, Subcommand)]
enum InspectKind {
    /// Decode a DOCX archive and print its OPC part index.
    Docx {
        /// Path to a `.docx` file.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("panduck: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Version => cmd_version(),
        Command::Convert { from, to, input, output } => cmd_convert(&from, &to, &input, output.as_deref()),
        Command::Project { from, input, output_dir } => cmd_project(&from, &input, &output_dir),
        Command::Inspect { kind } => cmd_inspect(kind),
    }
}

fn cmd_version() -> Result<(), String> {
    println!("panduck {}", env!("CARGO_PKG_VERSION"));
    println!("readers:  {}", panduck_convert::READERS.join(", "));
    println!("writers:  {}", panduck_convert::WRITERS.join(", "));
    let routes = panduck_convert::supported_routes();
    println!("routes:   {}", routes.len());
    for (from, to) in routes {
        println!("  {from} → {to}");
    }
    Ok(())
}

fn cmd_convert(from: &str, to: &str, input: &Path, output: Option<&Path>) -> Result<(), String> {
    if !supports_route(from, to) {
        return Err(format!("unsupported route: {from} → {to}"));
    }
    let output_value = convert_file(from, to, input).map_err(|e| e.to_string())?;

    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => derive_output_path(input, to),
    };
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }

    match to {
        "markdown" => fs::write(&out_path, &output_value.markdown),
        "html" => fs::write(&out_path, output_value.html.as_deref().unwrap_or("")),
        "docx" | "pdf" => fs::write(&out_path, output_value.binary.as_deref().unwrap_or(&[])),
        _ => unreachable!("supported route already filtered"),
    }
    .map_err(|e| format!("write {}: {e}", out_path.display()))?;

    println!("wrote {}", out_path.display());
    if output_value.loss_count > 0 {
        let loss_path = out_path.with_extension("report.json");
        fs::write(&loss_path, &output_value.report_json)
            .map_err(|e| format!("write report {}: {e}", loss_path.display()))?;
        eprintln!("warning: {} semantic loss markers recorded → {}", output_value.loss_count, loss_path.display());
    }
    Ok(())
}

fn cmd_project(from: &str, input: &Path, output_dir: &Path) -> Result<(), String> {
    if !supports_markdown_project_source(from) {
        return Err(format!("{from} does not support Markdown project output"));
    }
    fs::create_dir_all(output_dir).map_err(|e| format!("mkdir {}: {e}", output_dir.display()))?;
    let (output, published) = convert_markdown_project_file(from, input, output_dir)
        .map_err(|e| e.to_string())?;

    println!("published Markdown project → {}", published.output_dir.display());
    println!("  index: {}", published.index_path.display());
    println!("  assets: {} file(s)", output.published_assets.len());
    if !output.published_chapters.is_empty() {
        println!("  chapters: {} file(s)", output.published_chapters.len());
    }
    if !output.unresolved_assets.is_empty() {
        eprintln!("  unresolved assets: {} source(s)", output.unresolved_assets.len());
    }
    if output.loss_count > 0 {
        let report_path = output_dir.join("panduck.report.json");
        fs::write(&report_path, &output.report_json)
            .map_err(|e| format!("write report {}: {e}", report_path.display()))?;
        eprintln!("  warning: {} semantic loss markers → {}", output.loss_count, report_path.display());
    }
    Ok(())
}

fn cmd_inspect(kind: InspectKind) -> Result<(), String> {
    match kind {
        InspectKind::Docx { file } => {
            let index = inspect_docx_index(&file).map_err(|e| e.to_string())?;
            println!("format:   {}", index.format);
            println!("outer:    {}", index.outer);
            println!("inner:    {}", index.inner);
            println!("parts:    {} member(s)", index.parts.len());
            for part in &index.parts {
                println!("  {part}");
            }
            Ok(())
        }
    }
}

fn derive_output_path(input: &Path, to: &str) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match to {
        "markdown" => "md",
        "html" => "html",
        "docx" => "docx",
        "pdf" => "pdf",
        other => other,
    };
    input.with_file_name(format!("{stem}.{ext}"))
}
