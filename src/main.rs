mod core;
use anyhow::{Context, Result, bail};
use clap::Parser;
use core::Snippet;
use core::cli::Command;
use directories::ProjectDirs;
use std::env;
use std::path::PathBuf;

fn get_dirs() -> Result<ProjectDirs> {
    let dirs = ProjectDirs::from("com", "Nicolás Miranda", "Snipp")
        .context("Unable to fetch directories for the operative system")?;

    let saves_dir = dirs.data_dir().join("saves");
    std::fs::create_dir_all(&saves_dir)?;
    Ok(dirs)
}

fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let dirs = get_dirs()?;

    let saves_dir = dirs.data_dir().join("saves");

    let cli = core::cli::Cli::parse();

    match cli.command {
        Command::Create {
            name,
            description,
            source,
            output,
        } => {
            let source: PathBuf = source.unwrap_or(cwd);

            if !source.try_exists()? {
                bail!("The source at \"{}\" doesn't exists.", source.display());
            }

            let output: PathBuf = output.unwrap_or_else(|| saves_dir.join(&name));

            println!("Creating snippet...");
            let snipp = Snippet::create(name, description, source, output)?;
            let meta = snipp.metadata();

            println!("Created snippet: \"{}\"", meta.name());
        }
        Command::Show { path } => {
            if let Some(p) = path {
                let snipp = Snippet::load(p)?;
                let meta = snipp.metadata();

                println!("Name          : {}", meta.name());
                println!("File count    : {}", snipp.file_count()?);

                if let Some(date) = meta.creation_date() {
                    println!("Creation date : {}", date.format("%Y/%m/%d %H:%M"));
                }

                if let Some(desc) = meta.description() {
                    println!("Description   : {}", desc);
                }
            }
        }
    }

    Ok(())
}
