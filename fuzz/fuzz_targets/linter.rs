#![no_main]

use std::path::Path;

use comrak::Arena;

use mado::service::Linter;
use mado::{Config, Document, Rule};

use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: String| {
    let config = Config::default();
    let rules = Vec::<Rule>::from(&config.lint);
    let linter = Linter::new(rules);
    let arena = Arena::new();
    let path = Path::new("test.md").to_path_buf();
    // `Document::new` returns a `Result` it cannot produce an `Err` for: no
    // branch of it fails today. Unwrapping says so, rather than the target
    // growing a case the library does not have. Should `new` ever fail, a
    // panic here is what a fuzz target wants of it anyway. Allowed at the line
    // rather than by dropping the workspace's lints for the crate, the rest of
    // which are worth having.
    #[allow(clippy::unwrap_used)]
    let doc = Document::new(&arena, path, text).unwrap();
    let _ = linter.check(&doc);
});
