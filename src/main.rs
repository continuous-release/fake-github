use fake_github::FakeGitHub;
use std::io::{self, Read, Write};

fn main() {
    let mut fake = FakeGitHub::new();
    let url = fake.start().expect("failed to start fake GitHub");
    println!("{url}");
    let _ = io::stdout().flush();
    // Block until stdin closes (e.g. Ctrl-D) or the process is signalled.
    let mut buf = [0u8; 1];
    let _ = io::stdin().read(&mut buf);
    fake.stop();
}
