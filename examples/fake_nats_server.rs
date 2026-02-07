use std::io::{self, Write};
use std::time::Duration;

fn main() {
    let mut stdout = io::stdout();
    println!("fake_nats_server ready");
    let _ = stdout.flush();
    std::thread::sleep(Duration::from_secs(3));
}
