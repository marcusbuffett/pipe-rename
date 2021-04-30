use ansi_term::Colour;
use clap::{App, ArgMatches, Values};

use anyhow::{anyhow, Context};
use dialoguer::Select;
use shell_escape::escape;
use std::borrow::Cow;
use std::env;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;
use tempfile;

use thiserror::Error;

mod text_diff;
use text_diff::{calculate_text_diff, TextDiff};

#[derive(Debug, Clone)]
struct Rename {
    original: String,
    new: String,
}

impl Rename {
    fn pretty_diff(&self) -> impl Display {
        struct PrettyDiff(Rename);
        impl Display for PrettyDiff {
            fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
                let diff_changes = calculate_text_diff(&self.0.original, &self.0.new);

                // print old
                write!(f, "{}", Colour::Red.paint("- "))?;
                for change in &diff_changes {
                    match change {
                        TextDiff::Removed(old) => {
                            write!(f, "{}", Colour::Red.paint(old))?;
                        },
                        TextDiff::Unchanged(same) => {
                            write!(f, "{}", same)?;
                        },
                        _ => (),
                    }
                }
                writeln!(f)?;

                // print new
                write!(f, "{}", Colour::Green.paint("+ "))?;
                for change in &diff_changes {
                    match change {
                        TextDiff::New(new) => {
                            write!(f, "{}", Colour::Green.paint(new))?;
                        },
                        TextDiff::Unchanged(same) => {
                            write!(f, "{}", same)?;
                        },
                        _ => (),
                    }
                }

                Ok(())
            }
        }
        PrettyDiff(self.clone())
    }

    fn plain_diff(&self) -> impl Display {
        struct PlainDiff(Rename);
        impl Display for PlainDiff {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} -> {}", self.0.original, self.0.new)
            }
        }
        PlainDiff(self.clone())
    }
}
impl Display for Rename {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.plain_diff().fmt(f)
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
        .into_iter()
        .zip(new_lines)
        .filter_map(|(original, new)| {
            if original == new {
                None
            } else {
                Some(Rename {
                    original: original.to_string(),
                    new: new.to_string()
                })
            }
        })
        .collect();

    if renames.is_empty() {
        return Err(RenamerError::NoReplacementsFound);
    }

    Ok(renames)
}

fn get_matches() -> ArgMatches<'static> {
    App::new("renamer")
            .version(clap::crate_version!())
            .author("Marcus B. <me@mbuffett.com>")
            .about("Takes a list of files and renames/moves them, by piping them through an external editor")
            .arg(
                clap::Arg::with_name("rename-command")
                 .value_name("COMMAND")
                 .long("rename-command")
                 .short("c")
                 .help("Optionally set a custom rename command, like 'git mv'")
                 )
            .arg(
                clap::Arg::with_name("yes")
                 .long("yes")
                 .short("y")
                 .help("Answer all prompts with yes")
                 )
            .arg(
                clap::Arg::with_name("pretty-diff")
                 .long("pretty-diff")
                 .short("p")
                 .help("Prettify diffs")
                 )
            .arg(
                clap::Arg::with_name("files")
                 .value_name("FILES")
                 .multiple(true)
                 .help("The files to rename")
                 )
            .get_matches()
}

fn get_input(files: Option<Values>) -> anyhow::Result<Vec<String>> {
    if let Some(files) = files {
        return Ok(files.map(|f| f.to_string()).collect());
    }

    let input = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };
    return Ok(input.lines().map(|f| f.to_string()).collect());
}

fn get_input_files(files: Option<Values>) -> anyhow::Result<Vec<String>> {
    let mut input_files = get_input(files)?;
    // This is a special case where we want to dive into
    // a directory if it's the only argument.
    if input_files.len() == 1 && is_dir(&input_files[0]) {
        input_files = expand_dir(&input_files[0])?;
    }
    if input_files.is_empty() {
        return Err(anyhow!("No input files on stdin or as args. Aborting."));
    }

    Ok(input_files)
}

fn is_dir(path: &str) -> bool {
    match fs::metadata(&path) {
        Ok(stat) => stat.is_dir(),
        _ => false
    }
}

fn expand_dir(path: &str) -> anyhow::Result<Vec<String>, io::Error> {
    Ok(fs::read_dir(path)?
        .flatten()
        .filter_map(|e| e.path()
                .into_os_string()
                .into_string().ok())
        .collect())
}

fn open_editor(input_files: &Vec<String>) -> anyhow::Result<Vec<String>> {
    let mut tmpfile = tempfile::NamedTempFile::new().context("Could not create temp file")?;
    write!(tmpfile, "{}", input_files.join("\n"))?;
    let editor = env::var("EDITOR").unwrap_or("vim".to_string());
    tmpfile.seek(SeekFrom::Start(0))?;
    let child = Command::new(editor)
        .arg(tmpfile.path())
        .spawn()
        .context("Failed to execute editor process")?;

    child.wait_with_output()?;

    Ok(fs::read_to_string(&tmpfile)?
        .lines()
        .map(|f| f.to_string())
        .collect())
}

fn check_for_existing_files(replacements: &Vec<Rename>) -> anyhow::Result<()> {
    let replacements_over_existing_files: Vec<_> = replacements
        .iter()
        .filter(|replacement| Path::new(&replacement.new).exists())
        .collect();
    if !replacements_over_existing_files.is_empty() {
        println!("The following replacements overwrite existing files:");
        for replacement in &replacements_over_existing_files {
            println!("{}", Colour::Green.paint(replacement.to_string()));
        }
        println!();
        return Err(anyhow!("Refusing to overwrite existing files. Aborting."));
    }

    Ok(())
}

fn print_replacements(replacements: &Vec<Rename>, pretty: bool) {
    println!(
        "{}",
        Colour::Yellow.paint("The following replacements were found:")
    );
    println!();

    if pretty {
        let diff_output = replacements
            .iter()
            .map(|repl| repl.pretty_diff().to_string())
            .collect::<Vec<String>>()
            .join("\n\n"); // leave a blank line between pretty file diffs
        println!("{}", diff_output);
    } else {
        for replacement in replacements {
            println!("{}", Colour::Green.paint(replacement.to_string()));
        }
    }
    println!();
}

fn execute_renames(replacements: &Vec<Rename>, rename_command: Option<&str>) -> anyhow::Result<()> {
    for replacement in replacements {
        if let Some(cmd) = rename_command {
            subprocess::Exec::shell(format!(
                    "{} {} {}",
                    cmd,
                    escape(Cow::from(replacement.original.clone())),
                    escape(Cow::from(replacement.new.clone()))
            )).join()?;
        } else {
            fs::rename(&replacement.original, &replacement.new)?;
        }
    }

    Ok(())
}

fn prompt(yes: bool) -> anyhow::Result<&'static str> {
    let selections = vec!["Yes", "No", "Edit", "Reset"];

    if yes {
        return Ok(selections[0]);
    }

    let selection = Select::new()
        .with_prompt("Execute these renames?")
        .default(0)
        .items(&selections)
        .interact()?;

    Ok(selections[selection])
}

fn main() -> anyhow::Result<()> {
    let matches = get_matches();
    let input_files = get_input_files(matches.values_of("files"))?;
    let mut buffer = input_files.clone();

    loop {
        let new_files = open_editor(&buffer)?;
        let replacements = find_renames(&input_files, &new_files)?;
        println!();

        check_for_existing_files(&replacements)?;
        print_replacements(&replacements, matches.is_present("pretty-diff"));

        match prompt(matches.is_present("yes"))? {
            "Yes" => {
                execute_renames(&replacements, matches.value_of("rename-command"))?;
                break;
            },
            "No" => {
                println!("Aborting");
                break;
            },
            "Edit" => buffer = new_files.clone(),
            "Reset" => buffer = input_files.clone(),
            _ => unreachable!(),
        }
    }

    Ok(())
}
