use std::io::{self, BufRead, Seek, SeekFrom, Write};

fn write_formatted_binary<B: BufRead + Seek, W: Write>(
    mut s: B,
    nbytes_per_row: usize,
    mut output: W,
) -> io::Result<()> {
    let beginning_offset = s.seek(SeekFrom::Current(0))? as usize;

    for (i, byte) in s.bytes().enumerate() {
        let byte = byte?;

        let offset = beginning_offset + i;

        if i % nbytes_per_row == 0 {
            // Start of new line
            write!(output, "\n{:08x}: ", offset)?;
        }

        if i % 2 == 0 {
            write!(output, "{:02X}", byte)?;
        } else {
            write!(output, "{:02X} ", byte)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_printing_correctly() {
        let input = Cursor::new(vec![1, 2, 3]);
        let mut output = Vec::new();

        write_formatted_binary(input, 16, &mut output).unwrap();

        assert_eq!(output, b"\n00000000: 0102 03");
    }
}
