use std::sync::{Arc, Mutex};

use console::{clear_global_sink, ConsoleEvent, ConsoleSink, EventKind, Filter, Level, VecSink};
use panduck_convert::convert_bytes;
use serial_test::serial;

struct SharedSink(Arc<Mutex<VecSink>>);

impl ConsoleSink for SharedSink {
    fn emit(&mut self, event: &ConsoleEvent) {
        self.0.lock().expect("vec sink mutex poisoned").emit(event);
    }
}

#[test]
#[serial]
fn notedown_loss_emits_semantic_diagnostic_console_event() {
    clear_global_sink();
    let sink = Arc::new(Mutex::new(VecSink::new()));
    console::set_global_sink(Box::new(SharedSink(sink.clone())));
    console::set_global_filter(Filter::new(Level::Trace));

    let source = b"# Route\n\nBody.\n".to_vec();
    let output = convert_bytes("notedown", "markdown", "route.nd", source).expect("convert");
    assert!(output.loss_count > 0);

    let guard = sink.lock().expect("vec sink mutex poisoned");
    assert!(
        guard.events().iter().any(|event| {
            let payload = match event.kind() {
                EventKind::Diagnostic(payload) => payload,
                _ => return false,
            };
            payload.code().contains("partial_coverage")
                || payload.message().key() == "panduck.semantic.loss"
        }),
        "expected semantic loss diagnostic event",
    );

    clear_global_sink();
}
