use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rs-cleaner",
    version,
    about = "Clean unused files from projects"
)]
pub struct Cli {
    #[arg(help = "Path to directory to clean (defaults to current directory)")]
    pub path: Option<PathBuf>,

    #[arg(
        short = 'o',
        long = "older-than",
        help = "Only remove files older than given number of days",
        value_name = "DAYS"
    )]
    pub older_than: Option<u64>,

    #[arg(
        short = 'd',
        long = "depth",
        default_value_t = 2,
        help = "Maximum directory depth to search",
        value_name = "LEVEL"
    )]
    pub depth: usize,

    #[arg(short = 'y', long = "yes", help = "Automatically accept all prompts")]
    pub auto_accept: bool,

    #[arg(
        short = 'p',
        long,
        help = "Show what would be deleted without actually deleting"
    )]
    pub preview: bool,

    #[arg(short, long)]
    pub verbose: bool,
}
