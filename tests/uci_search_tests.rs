use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

#[test]
fn reports_mate_scores_in_moves() {
    for (fen, depth, expected_score) in [
        ("7k/8/5KQ1/8/8/8/8/8 w - - 0 1", 1, "mate 1"),
        ("7k/8/5K2/6Q1/8/8/8/8 b - - 0 1", 2, "mate -1"),
        ("7k/6Q1/5K2/8/8/8/8/8 b - - 1 1", 1, "mate 0"),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rusty_engine"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("engine should start");

        // Setting Hash waits for the search worker to finish before quit.
        write!(
            child.stdin.take().expect("stdin should be piped"),
            "position fen {fen}\ngo depth {depth}\nsetoption name Hash value 1\nquit\n"
        )
        .expect("UCI transcript should be written");

        let output = child.wait_with_output().expect("engine should exit");
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("engine output should be UTF-8");
        let depth_prefix = format!("info depth {depth} ");
        let score_field = format!(" score {expected_score} nodes ");
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with(&depth_prefix) && line.contains(&score_field)),
            "{fen}: expected {expected_score}, got {stdout}"
        );
    }
}

#[test]
fn pondering_waits_for_ponderhit_before_reporting_bestmove() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rusty_engine"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("engine should start");
    let mut input = child.stdin.take().expect("stdin should be piped");
    let output = child.stdout.take().expect("stdout should be piped");
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let timeout = Duration::from_secs(5);
    let result = (|| {
        write!(input, "position startpos\ngo ponder depth 1\n").ok()?;
        loop {
            if rx.recv_timeout(timeout).ok()?.starts_with("info depth 1 ") {
                break;
            }
        }
        if rx.recv_timeout(Duration::from_millis(50)) != Err(mpsc::RecvTimeoutError::Timeout) {
            return None;
        }
        writeln!(input, "ponderhit").ok()?;
        rx.recv_timeout(timeout)
            .ok()
            .filter(|line| line.starts_with("bestmove ") && !line.starts_with("bestmove 0000"))
    })();

    if result.is_some() {
        writeln!(input, "quit").expect("engine should receive quit");
    } else {
        let _ = child.kill();
    }
    let status = child.wait().expect("engine should exit");
    reader.join().expect("reader should exit");
    assert!(
        result.is_some() && status.success(),
        "expected bestmove after ponderhit: {result:?}"
    );
}
