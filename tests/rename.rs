mod run;

use run::TestCase;

#[test]
fn test_one_file() -> anyhow::Result<()> {
    let mut test_case = TestCase::new()?;
    test_case.replace("1", "2")?;

    test_case.assert_run()?;
    test_case.assert_renamed()?;

    Ok(())
}

#[test]
fn test_multiple_files() -> anyhow::Result<()> {
    let mut test_case = TestCase::new()?;
    test_case.replace("1", "2")?;
    test_case.replace("3", "4")?;
    test_case.replace("5", "6")?;

    test_case.assert_run()?;
    test_case.assert_renamed()?;

    Ok(())
}

#[test]
fn test_with_space() -> anyhow::Result<()> {
    let mut test_case = TestCase::new()?;
    test_case.replace("1 1", "2 2")?;

    test_case.assert_run()?;
    test_case.assert_renamed()?;

    Ok(())
}

#[test]
fn test_option() -> anyhow::Result<()> {
    let mut test_case = TestCase::new()?;
    test_case.replace("--cold-be-an-option", "--another-flag")?;

    test_case.assert_run()?;
    test_case.assert_renamed()?;

    Ok(())
}
