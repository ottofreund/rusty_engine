use std::process::Command;

const EXPECTED_NODES: u64 = 230_771;

fn run_bench() -> (u64, u64, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_rusty_engine"))
        .arg("bench")
        .output()
        .expect("bench command should start");

    assert!(
        output.status.success(),
        "bench command should exit successfully: {output:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("bench output should be UTF-8");
    let final_line = stdout
        .lines()
        .last()
        .expect("bench should print a final result line");
    let fields: Vec<&str> = final_line.split_whitespace().collect();

    assert_eq!(fields.len(), 4, "unexpected bench line: {final_line}");
    assert_eq!(fields[1], "nodes", "unexpected bench line: {final_line}");
    assert_eq!(fields[3], "nps", "unexpected bench line: {final_line}");

    let nodes = fields[0]
        .parse::<u64>()
        .expect("bench node count should be an integer");
    let nps = fields[2]
        .parse::<u64>()
        .expect("bench NPS should be an integer");
    (nodes, nps, stdout)
}

#[test]
fn command_line_bench_is_openbench_compatible_and_repeatable() {
    let (first_nodes, first_nps, first_stdout) = run_bench();
    let (second_nodes, second_nps, second_stdout) = run_bench();

    assert_eq!(first_nodes, EXPECTED_NODES, "first output:\n{first_stdout}");
    assert_eq!(
        second_nodes, EXPECTED_NODES,
        "second output:\n{second_stdout}"
    );
    assert_eq!(first_nodes, second_nodes);
    assert!(first_nps > 0);
    assert!(second_nps > 0);
}
