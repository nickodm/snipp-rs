use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

const STYLES: Styles = Styles::styled()
    .usage(AnsiColor::Blue.on_default().bold())
    .header(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Magenta.on_default().dimmed())
    .literal(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(version, about, styles = STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new snippet
    Create {
        /// The snippet's name.
        #[arg(short, long)]
        name: String,
        /// A description for the snippet.
        #[arg(short, long)]
        description: Option<String>,
        /// The path of the directory to convert to a snippet.
        /// Defaults to the current working directory.
        #[arg(short = 'p', long)]
        source: Option<PathBuf>,
        /// Specify an output for the snippet
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List all the saved snippets.
    List,
    /// Show information about a snippet.
    Show {
        /// The path of the snippet to show information about.
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
}
