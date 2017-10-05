use std::fs;
use std::io::Read;

use logger::{clear_global_sink, emit, LogEvent, Level};
use panduck_napi::install_logger_log_file;
use serial_test::serial;

#[test]
#[serial]
fn install_logger_log_file_sets_global_sink() {
    clear_global_sink();
    let path = std::env::temp_dir().join(format!("panduck-napi-logger-{}.jsonl", std::process::id()));
    install_logger_log_file(path.to_string_lossy().to_string()).expect("install log file");

    emit(
        LogEvent::log(Level::Info, "panduck-napi")
            .field("message", logger::FieldValue::str("ready"))
            .build(),
    );

    clear_global_sink();

    let mut file = fs::File::open(&path).expect("open log file");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("read log file");
    assert!(text.contains("\"kind\":\"log\""));
    assert!(text.contains("\"target\":\"panduck-napi\""));

    let _ = fs::remove_file(path);
}
