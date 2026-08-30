use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn supports_hash_option_in_a_uci_session() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rusty_engine"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("engine should start");

    {
        let mut stdin = child.stdin.take().expect("stdin should be piped");
        stdin
            .write_all(
                b"uci\nsetoption name Hash value 1\nsetoption name Hash value invalid\nsetoption\nisready\nquit\n",
            )
            .expect("UCI transcript should be written");
    }

    let output = child.wait_with_output().expect("engine should exit");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("engine output should be UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    let id_name_index = lines
        .iter()
        .position(|line| line.starts_with("id name "))
        .expect("engine id should be emitted");
    let hash_option_index = lines
        .iter()
        .position(|line| *line == "option name Hash type spin default 16 min 1 max 32768")
        .expect("Hash option should be emitted");
    let uciok_index = lines
        .iter()
        .position(|line| *line == "uciok")
        .expect("uciok should be emitted");

    assert!(id_name_index < hash_option_index);
    assert!(hash_option_index < uciok_index);
    assert!(lines.contains(&"readyok"));
    assert!(!stdout.contains("Invalid command"));
}
