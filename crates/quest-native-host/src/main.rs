use std::io::{self, BufReader, BufWriter};

#[cfg(not(windows))]
use quest_native_host::handle_message;
#[cfg(windows)]
use quest_native_host::{browser_from_host_name, configured_database, handle_message_with_browser};
use quest_native_host::{read_frame, write_frame, NativeResponse};
use quest_storage::Storage;

fn main() {
    if let Err(error) = run() {
        eprintln!("quest native host: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let (storage, browser_kind) = {
        let executable = std::env::current_exe()?;
        let browser = executable
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(browser_from_host_name);
        let storage = match configured_database(&executable)? {
            Some(database) => Storage::open(database)?,
            None => Storage::open_default()?,
        };
        (storage, browser)
    };
    #[cfg(not(windows))]
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
        #[cfg(windows)]
        let response = handle_message_with_browser(&storage, &message, browser_kind);
        #[cfg(not(windows))]
        let response = handle_message(&storage, &message);
        let response = serde_json::to_vec(&response)?;
        write_frame(&mut writer, &response)?;
    }
}
