mod run;

use run::{run_with_env, TestCase, EMPTY};

#[test]
fn test_no_input() -> anyhow::Result<()> {
    let assert = run_with_env(EMPTY, EMPTY, true)?;
    assert
        .failure()
        .stdout("")
        .stderr("Error: No input files on stdin or as args. Aborting.\n");
    Ok(())
}

#[test]
fn test_no_replacements() -> anyhow::Result<()> {
    let assert = run_with_env(&["test-1"], &["test-1"], true)?;
    assert
        .failure()
        .stdout("")
        .stderr("Error: No replacements found\n");
    Ok(())
}

#[test]
fn test_unequal_lines() -> anyhow::Result<()> {
    let assert = run_with_env(&["test-1", "test-2"], &["test-3"], true)?;
    assert
        .failure()
        .stdout("")
        .stderr("Error: Unequal number of files\n");
    Ok(())
}

#[test]
fn test_rename() -> anyhow::Result<()> {
    let mut test_case = TestCase::new()?;
    test_case.replace("1", "2")?;
    test_case.replace("2", "3")?;

    let assert = test_case.run()?;
    assert
        .failure()
        .stderr("Error: Refusing to overwrite existing files. Aborting.\n");

    // TODO: assert stdout
    // TODO: assert that nothing has been renamed

    Ok(())
}

#[test]
#[should_panic(expected = "assertion failed: `(left == right)`")]
fn test_dot() {
    let _ = run_with_env(&["."], &["."], false);
}

#[test]
#[should_panic(expected = "assertion failed: `(left == right)`")]
fn test_dotdot() {
    let _ = run_with_env(&[".."], &[".."], false);
}
