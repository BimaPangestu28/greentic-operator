use std::io::{self, Write};

fn main() {
    println!("ready");
    let _ = io::stdout().flush();

    let seconds = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<u64>().ok())
        .unwrap_or(2);
    std::thread::sleep(std::time::Duration::from_secs(seconds));
}
