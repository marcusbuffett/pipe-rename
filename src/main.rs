use ansi_term::Colour;
use clap::Parser;

use anyhow::{bail, Context};
use dialoguer::Select;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

mod text_diff;
use text_diff::{calculate_text_diff, TextDiff};

#[derive(Parser, Debug)]
#[clap(
    version = env!("CARGO_PKG_VERSION"),
    author = "Marcus B. <me@mbuffett.com>",
    about = "https://github.com/marcusbuffett/pipe-rename",
    long_about = "Takes a list of files and renames/moves them by piping them through an external editor"
)]
struct Opts {
    #[clap(name = "FILES")]
    files: Vec<String>,

    /// Use a custom rename command, like 'git mv'
    #[clap(short = 'c', long, value_name = "COMMAND")]
    rename_command: Option<String>,

    /// Specify what editor to use
    #[clap(short = 'e', long)]
    editor: Option<String>,

    /// Prettify diffs
    #[clap(short, long)]
    pretty_diff: bool,

    /// Answer all prompts with yes
    #[clap(short = 'y', long = "yes")]
    assume_yes: bool,

    /// Overwrite existing files
    #[clap(short, long)]
    force: bool,

    /// Undo the previous renaming operation
    #[clap(short, long)]
    undo: bool,

    /// Only rename filenames
    #[clap(short = 'n', long)]
    filenames_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Rename {
    original: PathBuf,
    new: PathBuf,
}

impl Rename {
    fn new(original: &str, new: &str) -> Self {
        // Expand ~ if applicable.
        let mut new = new.to_string();
        if let Ok(home) = env::var("HOME") {
            if new.starts_with("~/") {
                new = new.replacen('~', &home, 1);
            }
        }

        Rename {
            original: original.into(),
            new: new.into(),
        }
    }

    fn pretty_diff(&self) -> impl Display {
        struct PrettyDiff(Rename);
        impl Display for PrettyDiff {
            fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
                let diff_changes = calculate_text_diff(
                    &self.0.original.display().to_string(),
                    &self.0.new.display().to_string(),
                );

                // Print old.
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

                // Print new.
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
                write!(
                    f,
                    "{} -> {}",
                    self.0.original.display(),
                    self.0.new.display()
                )
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
    #[error("No replacements found.")]
    NoReplacementsFound,
    #[error("Unequal number of files.")]
    UnequalLines,
    #[error("Duplicate output files.")]
    DuplicateOutput,
}

fn find_renames(old_lines: &[String], new_lines: &[String]) -> Result<Vec<Rename>, RenamerError> {
    if old_lines.len() != new_lines.len() {
        return Err(RenamerError::UnequalLines);
    }
    let renames: Vec<_> = old_lines
        .iter()
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

    has_duplicate_renames(&renames)?;

    Ok(renames)
}

/// Check for duplicate new files.
fn has_duplicate_renames(replacements: &[Rename]) -> Result<(), RenamerError> {
    let mut set = HashSet::new();

    for item in replacements {
        if !set.insert(item.new.clone()) {
            return Err(RenamerError::DuplicateOutput);
        }
    }

    Ok(())
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
        bail!("No input files on stdin or as args.");
    }

    Ok(input.lines().map(|f| f.to_string()).collect())
}

fn get_input_files(files: Vec<String>) -> anyhow::Result<Vec<String>> {
    let mut input_files = get_input(files)?;
    // This is a special case where we want to expand `.` and `..`.
    let dots = &[".", ".."];
    if input_files.len() == 1 && dots.contains(&input_files[0].as_str()) {
        input_files = expand_dir(&input_files[0])?;
    }
    if input_files.is_empty() {
        bail!("No input files on stdin or as args.");
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

/// Split path into directory path and filename.
fn path_and_file_name(line: &String) -> Option<(PathBuf, String)> {
    let path = PathBuf::from(line);
    let dirname = path.parent().map(PathBuf::from);
    let file_name = path
        .file_name()
        .and_then(|f| f.to_os_string().into_string().ok());

    match (dirname, file_name) {
        (Some(d), Some(f)) => Some((d, f)),
        _ => None,
    }
}

fn open_editor(
    input_files: &[String],
    editor_string: &str,
    filenames_only: bool,
) -> anyhow::Result<Vec<String>> {
    let mut tmpfile = tempfile::Builder::new()
        .prefix("renamer-")
        .suffix(".txt")
        .tempfile()
        .context("Could not create temp file")?;

    let mut components: Vec<(PathBuf, String)> = vec![];

    if filenames_only {
        components = input_files.iter().filter_map(path_and_file_name).collect();

        write!(
            tmpfile,
            "{}",
            components
                .iter()
                .map(|(_, filename)| filename.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )?;
    } else {
        write!(tmpfile, "{}", input_files.join("\n"))?;
    }

    let editor_parsed = shell_words::split(editor_string)
        .expect("failed to parse command line flags in EDITOR command");
    tmpfile.seek(SeekFrom::Start(0))?;
    let child = Command::new(&editor_parsed[0])
        .args(&editor_parsed[1..])
        .arg(tmpfile.path())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to execute editor command: '{}'",
                shell_words::join(editor_parsed)
            )
        })?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!("Editor terminated unexpectedly.");
    }

    let changes: Vec<_> = fs::read_to_string(&tmpfile)?
        .lines()
        .map(|f| f.to_string())
        .collect();

    // Add the path back to the filename.
    if filenames_only {
        return Ok(zip(components, changes)
            .map(|(parts, file_name)| parts.0.join(file_name).display().to_string())
            .collect());
    }

    Ok(changes)
}

fn check_for_existing_files(replacements: &[Rename], force: bool) -> anyhow::Result<()> {
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
        bail!("Refusing to overwrite existing files.");
    }

    Ok(())
}

fn check_input_files(input_files: &[String]) -> anyhow::Result<()> {
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
        bail!("Nonexistent input files.");
    }

    let mut set = HashSet::new();

    for item in input_files {
        if !set.insert(item.clone()) {
            bail!("Duplicate input files.");
        }
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
            .join("\n\n"); // Leave a blank line between pretty file diffs
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
            let cmd_parsed = shell_words::split(cmd)
                .expect("failed to parse command line flags in rename command");
            subprocess::Exec::cmd(&cmd_parsed[0])
                .args(&cmd_parsed[1..])
                .arg(&replacement.original)
                .arg(&replacement.new)
                .join()?;
        } else {
            match fs::rename(&replacement.original, &replacement.new) {
                Ok(()) => (),
                // If renaming fails, try creating parent directories and try again.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let dir = &replacement.new.parent();
                    if let Some(dir) = dir {
                        fs::create_dir_all(dir)?;
                        fs::rename(&replacement.original, &replacement.new)?;
                    }
                }
                Err(e) => return Err(e.into()),
            };
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
        .items(selections)
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

fn make_absolute(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_relative() {
        Ok(std::env::current_dir()?.join(path))
    } else {
        Ok(path)
    }
}

fn write_undo_renames(backup_file: PathBuf, replacements: Vec<Rename>) -> anyhow::Result<()> {
    let undo_replacements = replacements
        .into_iter()
        .map(|r| {
            // Make paths absolute to that undo does not depend on CWD.
            let original = make_absolute(r.original)?;
            let new = make_absolute(r.new)?;

            Ok(Rename {
                // Swap original and new to get undo replacements.
                original: new,
                new: original,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let file = fs::File::create(backup_file)?;
    serde_json::to_writer(file, &undo_replacements)?;
    Ok(())
}

fn load_undo_renames(backup_file: PathBuf) -> anyhow::Result<Vec<Rename>> {
    let file = fs::File::open(&backup_file);
    let file = match file {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("No undo information found.")
        }
        Err(e) => return Err(e.into()),
    };
    let undo_replacements: Vec<Rename> = serde_json::from_reader(file)?;
    for replacement in &undo_replacements {
        if !replacement.original.exists() {
            bail!(
                "Undo not possible. \"{}\" is missing.",
                replacement.original.display()
            );
        }
    }
    fs::remove_file(backup_file)?;
    Ok(undo_replacements)
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse_from(wild::args());
    let backup_file = std::env::temp_dir().join("pipe-renamer_undo.json");

    if opts.undo {
        let replacements = load_undo_renames(backup_file)?;
        execute_renames(&replacements, opts.rename_command)?;
        println!("Restored {} files.", replacements.len());
        return Ok(());
    }

    let input_files = get_input_files(opts.files)?;
    check_input_files(&input_files)?;

    let editor = {
        let default_editor = if cfg!(windows) { "notepad.exe" } else { "vim" };
        opts.editor
            .unwrap_or_else(|| env::var("EDITOR").unwrap_or(default_editor.to_string()))
    };

    let mut buffer = input_files.clone();

    loop {
        let new_files = open_editor(&buffer, &editor, opts.filenames_only)?;
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
                write_undo_renames(backup_file, replacements)?;
                break;
            }
            MenuItem::No => {
                println!("Aborting.");
                break;
            }
            MenuItem::Edit => buffer = new_files.clone(),
            MenuItem::Reset => buffer = input_files.clone(),
        }
    }

    Ok(())
}
