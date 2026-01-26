use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use surveyhero::txt_writer::md_to_txt;

#[derive(Parser)]
/// Convert a Markdown file with questions into a txt file that can be imported into SurveyHero.
pub struct Cli {
    /// Path to the input Markdown file
    pub source: PathBuf,

    /// Path to the output txt file
    #[clap(short, long)]
    pub dist: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    md_to_txt(&cli.source, &cli.dist)
}
