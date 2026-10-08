use std::io::{self, BufReader, BufWriter};

use quest_native_host::{handle_message, read_frame, write_frame, NativeResponse};
use quest_storage::Storage;

fn main() {
    if let Err(error) = run() {
        eprintln!("quest native host: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let storage = Storage::open_default()?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    loop {
        let message = match read_frame(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => return Ok(()),
            Err(error) => {
                let response = serde_json::to_vec(&NativeResponse::error(error.to_string()))?;
                write_frame(&mut writer, &response)?;
                return Err(error.into());
            }
        };
        let response = serde_json::to_vec(&handle_message(&storage, &message))?;
        write_frame(&mut writer, &response)?;
    }
}
