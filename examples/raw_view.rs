use std::io::{Read, Write};

fn print_slice(s: &[u8], nbytes_per_row: usize) {
    let mut row = 0;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    'outer: loop {
        // Print address
        let addr = row * nbytes_per_row;
        write!(lock, "{:08X}: ", addr).unwrap();

        for byte_pos in 0..nbytes_per_row {
            let idx = addr + byte_pos;
            if idx >= s.len() {
                // EOF
                break 'outer;
            }

            let value = s[idx];

            if byte_pos == (nbytes_per_row - 1) || byte_pos % 2 == 0 {
                write!(lock, "{:02X}", value).unwrap();
            } else {
                write!(lock, "{:02X} ", value).unwrap();
            }
        }
        writeln!(lock).unwrap();

        row += 1;
    }

    writeln!(lock).unwrap();
}

fn main() {
    let mut file = std::fs::File::open("target/debug/examples/raw_view").unwrap();
    let mut buf = Vec::new();
    let n = file.read_to_end(&mut buf).unwrap();

    let view = &buf[buf.len() - 24..];

    print_slice(view, 16);
}
