use std::sync::{Arc, Mutex};

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
fn notedown_loss_emits_semantic_diagnostic_log_event() {
    clear_global_sink();
    let sink = Arc::new(Mutex::new(VecSink::new()));
    boost_log::set_global_sink(Box::new(SharedSink(sink.clone())));
    boost_log::set_global_filter(Filter::new(Level::Trace));

    let source = b"# Route\n\nBody.\n".to_vec();
    let output = convert_bytes("notedown", "markdown", "route.nd", source, None).expect("convert");
    assert!(output.loss_count > 0);

    let guard = sink.lock().expect("vec sink mutex poisoned");
    assert!(
        guard.events().iter().any(|event| {
            let payload = match event.kind() {
                EventKind::Diagnostic(payload) => payload,
                _ => return false,
            };
            payload.code().contains("partial_coverage") || payload.message().key() == "panduck.semantic.loss"
        }),
        "expected semantic loss diagnostic event",
    );

    clear_global_sink();
}
