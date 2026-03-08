use anyhow::Result;

pub fn run(shell: &str) -> Result<()> {
    let content = match shell {
        "zsh" => include_str!("../../shell/ccam.zsh"),
        "bash" => include_str!("../../shell/ccam.bash"),
        "fish" => include_str!("../../shell/ccam.fish"),
        other => anyhow::bail!(
            "unsupported shell: '{}'. Choose from: zsh, bash, fish",
            other
        ),
    };
    print!("{}", content);
    Ok(())
}
