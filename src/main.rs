mod core;
use anyhow::{Context, Result, bail};
use clap::Parser;
use core::Snippet;
use core::cli::Command;
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

fn get_dirs() -> Result<ProjectDirs> {
    let dirs = ProjectDirs::from("com", "Nicolás Miranda", "Snipp")
        .context("Unable to fetch directories for the operative system")?;

    let saves_dir = dirs.data_dir().join("saves");
    std::fs::create_dir_all(&saves_dir)?;
    Ok(dirs)
}

fn by_name(saves_dir: &PathBuf, name: &String) -> Result<Snippet> {
    let path = saves_dir.join(name).with_extension("snipp");

    if !path.try_exists()? {
        bail!("Snippet not found.")
    }

    let snipp = Snippet::load(&path)?;
    Ok(snipp)
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

        Command::Export { name, output } => {
            let output = match output {
                Some(o) if o.is_file() => o,
                Some(o) => o.join(&name).with_extension(Snippet::SUFFIX),
                None => env::current_dir()?
                    .join(&name)
                    .with_extension(Snippet::SUFFIX),
            };

            let path = saves_dir.join(&name).with_extension(Snippet::SUFFIX);

            if !path.try_exists()? {
                bail!("Snippet not found.");
            }

            fs::copy(path, &output)?;
            println!("Exported snippet to \"{}\"", output.display());
        }

        Command::Deploy { name, output } => {
            let output = match output {
                Some(p) => p,
                None => env::current_dir()?,
            };

            let snipp = by_name(&saves_dir, &name)?;
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
            let snipp = by_name(&saves_dir, &name)?;
            print_snippet(&snipp)?;
        }
    }

    Ok(())
}
