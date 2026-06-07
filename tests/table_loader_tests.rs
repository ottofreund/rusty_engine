use std::{fs, io::Write, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

use rusty_engine::search::table_loader::read_table_value_file;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("{name}_{nanos}.txt"))
}

#[test]
fn read_table_value_file_parses_real_table_and_reverses_rows() {
    let table_path = repo_root()
        .join("assets")
        .join("piece_square_tables")
        .join("knight.txt");
    let table_path_string = table_path
        .to_str()
        .expect("table path should be valid UTF-8")
        .to_owned();

    println!("table path string is: {}", table_path_string);

    let values = read_table_value_file(&table_path_string).expect("failed to read knight table");
    assert_eq!(values.len(), 64);

    // File starts with -50 and ends with -50; after reversal these remain at symmetric edges.
    assert_eq!(values[0], -50);
    assert_eq!(values[63], -50);

    // Spot-check that reversal changed row ordering as expected.
    // Original row 7 starts with -40, so reversed vector's row 0, col 0 is -40.
    assert_eq!(values[8], -40);
}

#[test]
fn read_table_value_file_errors_on_invalid_number() {
    let temp_file = unique_temp_file("table_loader_invalid");
    let mut file = fs::File::create(&temp_file).expect("failed to create temp table file");
    writeln!(file, "1,2,3").expect("failed to write invalid row");

    let temp_path = temp_file
        .to_str()
        .expect("temp file path should be valid UTF-8")
        .to_owned();
    let result = read_table_value_file(&temp_path);
    assert!(result.is_err());

    fs::remove_file(temp_file).expect("failed to remove temp table file");
}
