use std::io::{self, Write};

fn main() {
    let mut stdout = io::stdout();
    println!(
        "{{\"status\":\"ok\",\"argv\":{:?}}}",
        std::env::args().collect::<Vec<_>>()
    );
    let _ = stdout.flush();
}
