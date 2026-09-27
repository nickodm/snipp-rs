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

fn print_snippet(snipp: &Snippet) -> Result<()> {
    let meta = snipp.metadata();

    println!("Name          : {}", meta.name());
    println!("File count    : {}", snipp.file_count()?);

    if let Some(date) = meta.creation_date() {
        println!("Creation date : {}", date.format("%Y/%m/%d %H:%M"));
    }

    if let Some(desc) = meta.description() {
        println!("Description   : {}", desc);
    }

    Ok(())
}

fn main() -> Result<()> {
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
            let source: PathBuf = match source {
                Some(p) => p,
                None => env::current_dir()?,
            };

            if !source.try_exists()? {
                bail!("The source at \"{}\" doesn't exists.", source.display());
            }

            let output: PathBuf = output.unwrap_or_else(|| saves_dir.join(&name));

            println!("Creating snippet...");
            let snipp = Snippet::create(name, description, source, output)?;
            let meta = snipp.metadata();

            println!("Created snippet: \"{}\"", meta.name());
        }

        Command::Deploy { name, output } => {
            let output = match output {
                Some(p) => p,
                None => env::current_dir()?,
            };

            let path = saves_dir.join(name).with_extension("snipp");

            if !path.try_exists()? {
                bail!("Snippet not found.")
            }

            let snipp = Snippet::load(&path)?;
            let meta = snipp.metadata();
            snipp.deploy(&output)?;

            println!(
                "Deployed \"{}\" snippet to \"{}\".",
                meta.name(),
                output.display()
            );
        }

        Command::List => {
            let walk = walkdir::WalkDir::new(saves_dir);
            let mut counter: usize = 1;

            for entry in walk {
                let entry = entry?;
                let path = entry.path();

                if let Ok(snipp) = Snippet::load(path) {
                    let meta = snipp.metadata();
                    println!("[{:>2}] {}", counter, meta.name());
                    counter += 1;
                }
            }

            println!("---\nTotal: {}", counter - 1);
        }

        Command::Show { name } => {
            let path = saves_dir.join(name).with_extension("snipp");

            if !path.try_exists()? {
                bail!("Snippet not found.")
            }

            let snipp = Snippet::load(&path)?;
            print_snippet(&snipp)?;
        }
    }

    Ok(())
}
