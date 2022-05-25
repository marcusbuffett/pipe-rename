use ansi_term::Colour;
use clap::Parser;

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

#[derive(Parser, Debug)]
#[clap(
    version = "1.2",
    author = "Marcus B. <me@mbufett.com>",
    about = "https://github.com/marcusbuffett/pipe-rename"
)]
struct Opts {
    #[clap(name = "FILES")]
    files: Vec<String>,
    /// Optionally set a custom rename command, like 'git mv'
    #[clap(short = 'c', long)]
    rename_command: Option<String>,
    /// Prettify diffs
    #[clap(short, long)]
    pretty_diff: bool,
    /// Answer all prompts with yes
    #[clap(short = 'y', long = "yes")]
    assume_yes: bool,
    /// Overwrite existing files
    #[clap(short, long)]
    force: bool,
}

#[derive(Debug, Clone)]
struct Rename {
    original: String,
    new: String,
}

impl Rename {
    fn new(original: &str, new: &str) -> Self {
        // Expand ~ if applicable.
        let mut new = new.to_string();
        if let Ok(home) = env::var("HOME") {
            if &new[..2] == "~/" {
                new = new.replacen("~", &home, 1);
            }
        }

        Rename {
            original: original.to_string(),
            new,
        }
    }

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
                        }
                        TextDiff::Unchanged(same) => {
                            write!(f, "{}", same)?;
                        }
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
                        }
                        TextDiff::Unchanged(same) => {
                            write!(f, "{}", same)?;
                        }
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
                Some(Rename::new(original, new))
            }
        })
        .collect();

    if renames.is_empty() {
        return Err(RenamerError::NoReplacementsFound);
    }

    Ok(renames)
}

fn get_input(files: Vec<String>) -> anyhow::Result<Vec<String>> {
    if !files.is_empty() {
        return Ok(files);
    }

    let input = {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };
    if input.is_empty() {
        return Err(anyhow!("No input files on stdin or as args. Aborting."));
    }
    return Ok(input.lines().map(|f| f.to_string()).collect());
}

fn get_input_files(files: Vec<String>) -> anyhow::Result<Vec<String>> {
    let mut input_files = get_input(files)?;
    // This is a special case where we want to expand `.` and `..`.
    let dots = &[".", ".."];
    if input_files.len() == 1 && dots.contains(&input_files[0].as_str()) {
        input_files = expand_dir(&input_files[0])?;
    }
    if input_files.is_empty() {
        return Err(anyhow!("No input files on stdin or as args. Aborting."));
    }

    Ok(input_files)
}

fn expand_dir(path: &str) -> anyhow::Result<Vec<String>, io::Error> {
    Ok(fs::read_dir(path)?
        .filter_map(|e| {
            e.ok()
                .and_then(|e| e.path().into_os_string().into_string().ok())
        })
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

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!("Editor terminated unexpectedly. Aborting."));
    }

    Ok(fs::read_to_string(&tmpfile)?
        .lines()
        .map(|f| f.to_string())
        .collect())
}

fn check_for_existing_files(replacements: &Vec<Rename>, force: bool) -> anyhow::Result<()> {
    // Skip check if forcing renames.
    if force {
        return Ok(());
    }

    let replacements_over_existing_files: Vec<_> = replacements
        .iter()
        .filter(|replacement| Path::new(&replacement.new).exists())
        .collect();
    if !replacements_over_existing_files.is_empty() {
        println!("The following replacements overwrite existing files:");
        for replacement in &replacements_over_existing_files {
            println!("{}", Colour::Red.paint(replacement.to_string()));
        }
        println!();
        return Err(anyhow!("Refusing to overwrite existing files. Aborting."));
    }

    Ok(())
}

fn check_input_files(input_files: &Vec<String>) -> anyhow::Result<()> {
    let nonexisting_files: Vec<_> = input_files
        .iter()
        .filter(|input_file| !Path::new(input_file).exists())
        .collect();

    if !nonexisting_files.is_empty() {
        println!("The following input files do not exist:");
        for file in nonexisting_files {
            println!("{}", Colour::Red.paint(file));
        }
        println!();
        return Err(anyhow!("Nonexisting input files. Aborting."));
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

fn execute_renames(
    replacements: &Vec<Rename>,
    rename_command: Option<String>,
) -> anyhow::Result<()> {
    for replacement in replacements {
        if let Some(ref cmd) = rename_command {
            subprocess::Exec::shell(format!(
                "{} {} {}",
                cmd,
                escape(Cow::from(replacement.original.clone())),
                escape(Cow::from(replacement.new.clone()))
            ))
            .join()?;
        } else {
            fs::rename(&replacement.original, &replacement.new)?;
        }
    }

    Ok(())
}

fn prompt(selections: &[MenuItem], yes: bool) -> anyhow::Result<&MenuItem> {
    if yes {
        return Ok(&selections[0]);
    }

    let selection = Select::new()
        .with_prompt("Execute these renames?")
        .default(0)
        .items(&selections)
        .interact()?;

    Ok(&selections[selection])
}

enum MenuItem {
    /// Perform the current replacements
    Yes,
    /// Abort and do nothing
    No,
    /// Open the editor with the current replacements for edit
    Edit,
    /// Open the editor with the original names for edit
    Reset,
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuItem::Yes => f.write_str("Yes"),
            MenuItem::No => f.write_str("No"),
            MenuItem::Edit => f.write_str("Edit"),
            MenuItem::Reset => f.write_str("Reset"),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let input_files = get_input_files(opts.files)?;

    check_input_files(&input_files)?;

    let mut buffer = input_files.clone();

    loop {
        let new_files = open_editor(&buffer)?;
        let replacements = find_renames(&input_files, &new_files)?;
        println!();

        let check_existing = check_for_existing_files(&replacements, opts.force);

        let menu_options = match check_existing {
            Ok(()) => {
                print_replacements(&replacements, opts.pretty_diff);
                vec![MenuItem::Yes, MenuItem::No, MenuItem::Edit, MenuItem::Reset]
            }
            e @ Err(_) if opts.assume_yes => return e,
            Err(_) => vec![MenuItem::Edit, MenuItem::Yes, MenuItem::No, MenuItem::Reset],
        };

        match prompt(&menu_options, opts.assume_yes)? {
            MenuItem::Yes => {
                execute_renames(&replacements, opts.rename_command)?;
                break;
            }
            MenuItem::No => {
                println!("Aborting");
                break;
            }
            MenuItem::Edit => buffer = new_files.clone(),
            MenuItem::Reset => buffer = input_files.clone(),
        }
    }

    Ok(())
}
