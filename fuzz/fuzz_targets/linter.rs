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
    // A `new` that fails is something a fuzz target wants to hear about.
    #[allow(clippy::unwrap_used)]
    let doc = Document::new(&arena, path, text).unwrap();
    let _ = linter.check(&doc);
});
