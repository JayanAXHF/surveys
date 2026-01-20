/// Verify the contents of the Annual Rust Survey on SurveyHero.
#[derive(clap::Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: VerifierCmd,
}

#[derive(clap::Parser, Clone)]
pub struct SharedArgs {
    /// ID of the survey.
    #[clap(long)]
    pub survey_id: usize,
    /// Survey path. Corresponds to a Markdown file or a directory relative to `../surveys/`.
    #[clap(long)]
    pub path: String,
}

#[derive(clap::Parser, Clone)]
pub enum VerifierCmd {
    /// Shows a diff with the local Markdown files and the SurveyHero content.
    Check {
        #[clap(flatten)]
        shared: SharedArgs,
    },
    /// Downloads all Markdown files from SurveyHero (overwrites without asking)
    Download {
        #[clap(flatten)]
        shared: SharedArgs,
    },
}

impl VerifierCmd {
    pub fn shared(&self) -> &SharedArgs {
        match self {
            VerifierCmd::Check { shared } => shared,
            VerifierCmd::Download { shared } => shared,
        }
    }
}
