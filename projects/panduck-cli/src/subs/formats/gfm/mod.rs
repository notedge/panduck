use crate::{PanduckConfig, Result};
use clap::Parser;

/// Generate html file from given file
#[derive(Debug, Parser)]
pub struct GithubFavoredMD {
    #[clap(short, long)]
    /// Original file to be converted
    input: String,
    /// The location and name of the output file
    #[clap(short, long)]
    output: Option<String>,
    /// Whether to format the output
    #[clap(short, long)]
    pretty_print: bool,
    /// Whether to trust the html dangerous field in the file
    #[clap(short, long)]
    trust_html: bool,
}

impl GithubFavoredMD {
    pub fn apply_args(&self, cfg: &mut PanduckConfig) {
        cfg.html.trust_raw_html = self.trust_html;
    }

    pub fn dispatch(&self, cfg: &PanduckConfig) -> Result<()> {
        Ok(())
    }
}
