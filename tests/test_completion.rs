use std::process::Command;

#[test]
fn test_completion() {
    // TODO
    // The executable path, is present in the generated completion scripts.
    // To fix issues based on the paths to rt differ from on platform to
    // another, especially with CI. Obtain the directory containing rt, from
    // from_path. Append environment variable PATH so we can call the rt by
    // simply `rt` and not the full path.
    let rt_path = assert_cmd::cargo::cargo_bin!("rt").parent().unwrap();
    let path = format!(
        "{}:{}",
        rt_path.display(),
        std::env::var_os("PATH").unwrap().into_string().unwrap()
    );

    let mut cmd = Command::new("rt");
    cmd.env("COMPLETE", "zsh");
    cmd.env("PATH", path);
    let output = cmd.output().unwrap().stdout;

    insta::assert_snapshot!(String::from_utf8(output).unwrap());
}
