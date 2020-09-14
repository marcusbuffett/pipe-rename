use ansi_term::Colour;
use clap::App;

use anyhow::Context;
use dialoguer::Confirm;
use std::env;
use std::fmt::Display;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use thiserror::Error;

#[derive(Debug, Clone)]
struct Rename {
    original: String,
    new: String,
}

impl Display for Rename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.original, self.new)
    }
}

#[derive(Error, Debug, Clone)]
pub enum RenamerError {
    #[error("No replacements found")]
    NoReplacementsFound,
    #[error("Unequal number of files")]
    UnequalLines,
}

fn find_renames(
    old_lines: &Vec<String>,
    new_lines: &Vec<String>,
) -> Result<Vec<Rename>, RenamerError> {
    if old_lines.len() != new_lines.len() {
        return Err(RenamerError::UnequalLines);
    }
    let renames: Vec<_> = old_lines
        .iter()
        .zip(new_lines)
        .filter_map(|(old, new)| {
            if old.eq(new) {
                None
            } else {
                Some(Rename {
                    original: old.to_string(),
                    new: new.to_string(),
                })
            }
        })
        .collect();

    Ok(renames)
}

fn main() -> anyhow::Result<()> {
    let matches = App::new("renamer")
                          .version(clap::crate_version!())
                          .author("Marcus B. <me@mbuffett.com")
                          .about("Takes a list of files and renames/removes them, by piping them through an external editor")
                          .arg(
                              clap::Arg::with_name("rename-command")
                               .value_name("COMMAND")
                               .long("rename-command")
                               .short("c")
                               .help("Optionally set a custom rename command, like 'git mv'")
                               )
                          .arg(
                              clap::Arg::with_name("files")
                               .value_name("FILES")
                               .multiple(true)
                               .help("The files to rename")
                               )
                          .get_matches();
    let mut tmpfile = tempfile::NamedTempFile::new().context("Could not create temp file")?;
    let input_files: Vec<String> = if let Some(files) = matches.values_of("files") {
        files
            .into_iter()
            .map(|f| f.to_string())
            .collect::<Vec<String>>()
    } else {
        let input = {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        };
        if input.is_empty() {
            println!("No input files on stdin or as args. Aborting.");
            return Ok(());
        }
        input
            .lines()
            .map(|f| f.to_string())
            .collect::<Vec<String>>()
    };
    {
        write!(tmpfile, "{}", input_files.join("\n"))?;
        let editor = env::var("EDITOR").unwrap_or("vim".to_string());
        tmpfile.seek(SeekFrom::Start(0)).unwrap();
        let child = Command::new(editor)
            .arg(tmpfile.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .spawn()
            .context("Failed to execute editor process")?;

        child.wait_with_output()?;
    }
    let new_files: Vec<String> = fs::read_to_string(tmpfile)?
        .lines()
        .map(|f| f.to_string())
        .collect();
    let replacements = find_renames(&input_files, &new_files)?;
    if replacements.is_empty() {
        println!("No replacements found");
        return Err(RenamerError::NoReplacementsFound.into());
    }
    println!();
    let replacements_over_existing_files: Vec<_> = replacements
        .iter()
        .filter(|replacement| Path::new(&replacement.new).exists())
        .collect();
    if !replacements_over_existing_files.is_empty() {
        println!("The following replacements overwrite existing files:");
        for replacement in &replacements {
            println!("{}", Colour::Green.paint(replacement.to_string()));
        }
        println!();
        println!("Refusing to overwrite existing files. Aborting.");
        return Ok(());
    }
    println!(
        "{}",
        Colour::Yellow.paint("The following replacements were found")
    );
    println!();
    for replacement in &replacements {
        println!("{}", Colour::Green.paint(replacement.to_string()));
    }
    println!();
    if Confirm::new()
        .with_prompt("Execute these renames?")
        .interact()?
    {
        for replacement in &replacements {
            if let Some(cmd) = matches.value_of("rename-command") {
                subprocess::Exec::shell(format!(
                    "{} {} {}",
                    cmd, replacement.original, replacement.new
                ))
                .join()?;
            } else {
                fs::rename(&replacement.original, &replacement.new)?; // Rename a.txt to b.txt
            }
        }
    } else {
        println!("Aborting")
    }
    Ok(())
}
