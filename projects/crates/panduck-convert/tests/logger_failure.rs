use std::{
    io::Read,
    sync::{Arc, Mutex},
};

use boost_log::{EventKind, Filter, Level, LogEvent, LogSink, VecSink, clear_global_sink};
use panduck_convert::convert_bytes;
use serial_test::serial;

struct SharedSink(Arc<Mutex<VecSink>>);

impl LogSink for SharedSink {
    fn emit(&mut self, event: &LogEvent) {
        self.0.lock().expect("vec sink mutex poisoned").emit(event);
    }
}

#[test]
#[serial]
fn unsupported_route_emits_diagnostic_log_event() {
    clear_global_sink();
    let sink = Arc::new(Mutex::new(VecSink::new()));
    boost_log::set_global_sink(Box::new(SharedSink(sink.clone())));
    boost_log::set_global_filter(Filter::new(Level::Trace));

    let result = convert_bytes("xlsx", "markdown", "sheet.xlsx", Vec::new(), None);
    assert!(result.is_err());

    let guard = sink.lock().expect("vec sink mutex poisoned");
    assert_eq!(guard.events().len(), 1);
    let payload = match guard.events()[0].kind() {
        EventKind::Diagnostic(payload) => payload,
        _ => panic!("expected diagnostic log event"),
    };
    assert_eq!(payload.code(), "panduck.adapter.unsupported-format");

    clear_global_sink();
}

#[test]
#[serial]
fn unsupported_route_writes_json_lines_log_file() {
    clear_global_sink();
    let path = std::env::temp_dir().join(format!("panduck-convert-logger-failure-{}.jsonl", std::process::id()));
    boost_log::install_global_file_sink(&path).expect("install global file sink");

    let result = convert_bytes("xlsx", "markdown", "sheet.xlsx", Vec::new(), None);
    assert!(result.is_err());

    clear_global_sink();

    let mut file = std::fs::File::open(&path).expect("open log file");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("read log file");
    assert!(text.contains("\"kind\":\"diagnostic\""));
    assert!(text.contains("\"code\":\"panduck.adapter.unsupported-format\""));

    let _ = std::fs::remove_file(path);
}
