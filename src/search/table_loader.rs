use std::fs::File;
use std::io::{self, BufRead, BufReader};




pub fn read_table_value_file(file_path: &str) -> io::Result<Vec<i32>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut res: Vec<i32> = Vec::with_capacity(64);

    for line in reader.lines() {
        let line = line?;
        let value_row  = line
            .split(',')
            .map(|s| s.trim().parse::<i32>())
            .collect::<Result<Vec<_>, _>>()  // collect Results
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        value_row.iter().for_each(|v| {
            res.push(*v);
        });
    }
    res.reverse();
    return Ok(res);
}
