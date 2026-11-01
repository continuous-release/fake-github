use fake_github::FakeGitHub;
use std::env;
use std::io::{self, Read, Write};

fn main() {
    let mut args = env::args().skip(1);
    let mut state_path: Option<String> = None;
    let mut save_path: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--state" => state_path = args.next(),
            "--save" => save_path = args.next(),
            _ => {}
        }
    }

    let mut fake = FakeGitHub::new();
    if let Some(path) = &state_path {
        if let Err(e) = fake.load_state(std::path::Path::new(path)) {
            eprintln!("failed to load state: {e}");
        }
    }
    let url = fake.start().expect("failed to start fake GitHub");
    println!("{url}");
    let _ = io::stdout().flush();

    let mut buf = [0u8; 1];
    let _ = io::stdin().read(&mut buf);

    if let Some(path) = &save_path {
        if let Err(e) = fake.save_state(std::path::Path::new(path)) {
            eprintln!("failed to save state: {e}");
        }
    }
    fake.stop();
}