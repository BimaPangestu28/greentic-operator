use std::io::{self, Write};
use std::time::Duration;

fn main() {
    let mut stdout = io::stdout();
    println!("fake_gsm_gateway ready");
    let _ = stdout.flush();
    std::thread::sleep(Duration::from_secs(3));
}
