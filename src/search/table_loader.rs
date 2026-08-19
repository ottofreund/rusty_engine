use std::{fs, io};

pub fn read_table_value_file(file_path: &str) -> io::Result<Vec<i16>> {
    let contents = fs::read_to_string(file_path)?;
    parse_table_values(&contents)
}

pub fn parse_table_values(contents: &str) -> io::Result<Vec<i16>> {
    let mut res: Vec<i16> = Vec::with_capacity(64);

    for line in contents.lines() {
        let value_row = line
            .split(',')
            .filter(|el| !el.trim().is_empty())
            .map(|s| s.trim().parse::<i16>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if value_row.len() != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Row didn't have 8 elements.",
            ));
        }

        res.extend(value_row);
    }

    res.reverse();
    Ok(res)
}
