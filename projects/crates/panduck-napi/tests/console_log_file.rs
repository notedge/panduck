use std::fs;
use std::io::Read;

use console::{clear_global_sink, set_global_sink, VecSink};
use panduck_napi::install_console_log_file;
use serial_test::serial;

#[test]
#[serial]
fn install_console_log_file_sets_global_sink() {
    clear_global_sink();
    let path = std::env::temp_dir().join(format!("panduck-napi-log-{}.jsonl", std::process::id()));
    install_console_log_file(path.to_string_lossy().to_string()).expect("install log file");

    console::emit(
        console::ConsoleEvent::log(console::Level::Info, "panduck-napi")
            .field("message", console::FieldValue::str("ready"))
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

#[test]
#[serial]
fn vec_sink_still_works_after_clear() {
    let sink = VecSink::new();
    set_global_sink(Box::new(sink));
    clear_global_sink();
}
