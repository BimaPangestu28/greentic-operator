use std::io::{self, Write};
use std::time::Duration;

fn main() {
    let mut stdout = io::stdout();
    println!("cloudflared: starting");
    let _ = stdout.flush();
    std::thread::sleep(Duration::from_millis(100));
    println!("trycloudflare url https://example.trycloudflare.com");
    let _ = stdout.flush();
    std::thread::sleep(Duration::from_secs(3));
}
