use anyhow::Result;

pub fn run(shell: &str) -> Result<()> {
    let content = match shell {
        "zsh" => include_str!("../../shell/ccam.zsh"),
        "bash" => include_str!("../../shell/ccam.bash"),
        "fish" => include_str!("../../shell/ccam.fish"),
        other => anyhow::bail!(
            "지원하지 않는 쉘: '{}'. zsh, bash, fish 중 선택하세요.",
            other
        ),
    };
    print!("{}", content);
    Ok(())
}
