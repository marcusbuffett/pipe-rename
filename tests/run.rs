#![allow(dead_code)]

use std::collections;
use std::fs;
use std::path;

use anyhow::Context as _;

pub const EMPTY: &[&str] = &[];

pub fn renamer(editor: impl AsRef<path::Path>) -> anyhow::Result<assert_cmd::Command> {
    let editor_path = path::Path::new("tests/editors").join(editor.as_ref());
    anyhow::ensure!(
        editor_path.is_file(),
        "Editor {} not found",
        editor.as_ref().display()
    );
    let mut cmd =
        assert_cmd::Command::cargo_bin("renamer").context("Could not find renamer binary")?;
    cmd.arg("--yes");
    cmd.env("EDITOR", editor_path.canonicalize()?);
    Ok(cmd)
}

pub fn run_with_env(
    input: &[impl AsRef<str>],
    replacements: &[impl AsRef<str>],
    create_inputs: bool
) -> anyhow::Result<assert_cmd::assert::Assert> {
    let input: Vec<_> = input.iter().map(AsRef::as_ref).collect();
    let replacements: Vec<_> = replacements.iter().map(AsRef::as_ref).collect();

    let tempdir = tempfile::tempdir().context("Could not create tempdir")?;
    let input_file = tempdir.path().join("input");
    let output_file = tempdir.path().join("output");

    if create_inputs {
        for file in &input {
            fs::File::create(tempdir.path().join(file))?;
        }
    }

    fs::write(&output_file, replacements.join("\n"))
        .context("Could not write replacements to editor output file")?;

    let assert = renamer("env-editor.py")?
        .env("TEST_EDITOR_INPUT", &input_file)
        .env("TEST_EDITOR_OUTPUT", &output_file)
        .current_dir(tempdir.path())
        .write_stdin(input.join("\n"))
        .assert();

    let editor_input = if input_file.exists() {
        fs::read_to_string(&input_file).context("Could not read editor input file")?
    } else {
        String::new()
    };

    assert_eq!(input.join("\n"), editor_input);


    Ok(assert)
}

pub struct TestCase {
    dir: tempfile::TempDir,
    replacements: Vec<(String, String)>,
}

impl TestCase {
    pub fn new() -> anyhow::Result<TestCase> {
        let dir = tempfile::tempdir().context("Could not create tempdir")?;
        Ok(TestCase {
            dir,
            replacements: Vec::new(),
        })
    }

    pub fn replace(
        &mut self,
        old: impl Into<String>,
        new: impl Into<String>,
    ) -> anyhow::Result<()> {
        let old = old.into();
        let new = new.into();

        let old_file = self.dir.path().join(&old);
        fs::write(&old_file, &old).context("Could not write to test case file")?;

        self.replacements.push((old, new));

        Ok(())
    }

    pub fn input(&self) -> anyhow::Result<Vec<String>> {
        self.replacements
            .iter()
            .map(|(s, _)| self.dir.path().join(s))
            .map(|p| {
                p.to_str()
                    .map(ToOwned::to_owned)
                    .with_context(|| format!("Invalid input file name: {}", p.display()))
            })
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn replacements(&self) -> anyhow::Result<Vec<String>> {
        self.replacements
            .iter()
            .map(|(_, s)| self.dir.path().join(s))
            .map(|p| {
                p.to_str()
                    .map(ToOwned::to_owned)
                    .with_context(|| format!("Invalid output file name: {}", p.display()))
            })
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn run(&self) -> anyhow::Result<assert_cmd::assert::Assert> {
        run_with_env(&self.input()?, &self.replacements()?, false)
    }

    pub fn assert_run(&self) -> anyhow::Result<assert_cmd::assert::Assert> {
        let assert = self.run()?.success().stderr("");
        // TODO: assert stdout
        Ok(assert)
    }

    pub fn assert_renamed(&self) -> anyhow::Result<()> {
        let actual_files = self.get_files()?;
        let expected_files: collections::HashSet<_> =
            self.replacements.iter().map(|(_, s)| s).collect();
        assert_eq!(
            expected_files,
            actual_files
                .iter()
                .map(|s| s)
                .collect::<collections::HashSet<_>>()
        );

        for (old, new) in &self.replacements {
            let new_path = self.dir.path().join(&new);
            assert!(new_path.is_file(), "New file does not exist: {}", &new);
            let content = fs::read_to_string(&new_path).context("Could not read output file")?;
            assert_eq!(old, &content, "File {} has unexpected content", &new);
        }

        Ok(())
    }

    fn get_files(&self) -> anyhow::Result<Vec<String>> {
        let mut files = Vec::new();
        for file in self
            .dir
            .path()
            .read_dir()
            .context("Could not read tempdir")?
        {
            let file = file?;
            let file_name = file
                .file_name()
                .to_str()
                .map(|s| s.to_owned())
                .with_context(|| format!("Invalid file name: {:?}", file.file_name()))?;
            assert!(file.file_type()?.is_file(), "{} is not a file", &file_name);
            files.push(file_name);
        }
        Ok(files)
    }
}
