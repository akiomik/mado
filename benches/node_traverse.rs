use core::hint::black_box;

use comrak::Arena;
use comrak::Options;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use comrak::parse_document;
use criterion::{Criterion, criterion_group, criterion_main};

fn recursive<'a>(root: &'a AstNode<'a>, texts: &mut Vec<String>) {
    for node in root.children() {
        if let NodeValue::Text(text) = &node.data.borrow().value {
            texts.push(text.to_string());
        }

        recursive(node, texts);
    }
}

fn descendants<'a>(root: &'a AstNode<'a>) -> Vec<String> {
    let mut texts = vec![];

    for node in root.descendants() {
        if let NodeValue::Text(text) = &node.data.borrow().value {
            texts.push(text.to_string());
        }
    }

    texts
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_traverse");

    let markdown = include_str!("../README.md");
    let arena = Arena::new();
    let options = Options::default();
    let doc = parse_document(&arena, markdown, &options);

    group.bench_function("recursive", |b| {
        b.iter(|| recursive(black_box(doc), black_box(&mut vec![])));
    });

    group.bench_function("descendants", |b| {
        b.iter(|| descendants(black_box(doc)));
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
