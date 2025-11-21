use core::hint::black_box;
use std::sync::LazyLock;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::distr::SampleString as _;
use rand::distr::StandardUniform;
use regex::Regex;

static RE: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    #[allow(clippy::trivial_regex)]
    Regex::new("\t").unwrap()
});

fn regex_captures(s: &str) -> Option<(usize, usize)> {
    let mut locs = RE.capture_locations();
    RE.captures_read(&mut locs, s);
    if let Some((start_column, end_column)) = locs.get(0) {
        return Some((start_column + 1, end_column));
    }

    None
}

fn regex_find(s: &str) -> Option<(usize, usize)> {
    if let Some(m) = RE.find(s) {
        return Some((m.start() + 1, m.end()));
    }

    None
}

fn string_find(s: &str) -> Option<(usize, usize)> {
    if let Some(index) = s.find('\t') {
        let column = index + 1;
        return Some((column, column));
    }

    None
}

fn string_match_indices(s: &str) -> Option<(usize, usize)> {
    if let Some((index, _)) = s.match_indices('\t').next() {
        let column = index + 1;
        return Some((column, column));
    }

    None
}

fn chars_position(s: &str) -> Option<(usize, usize)> {
    if let Some(index) = s.chars().position(|c| c == '\t') {
        let column = index + 1;
        return Some((column, column));
    }

    None
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("hard_tab_search");

    for i in 1..=3 {
        let string = StandardUniform.sample_string(&mut rand::rng(), (i as usize).pow(i) * 100);

        group.bench_with_input(BenchmarkId::new("regex_captures", i), &string, |b, s| {
            b.iter(|| regex_captures(black_box(s)));
        });

        group.bench_with_input(BenchmarkId::new("regex_find", i), &string, |b, s| {
            b.iter(|| regex_find(black_box(s)));
        });

        group.bench_with_input(BenchmarkId::new("string_find", i), &string, |b, s| {
            b.iter(|| string_find(black_box(s)));
        });

        group.bench_with_input(
            BenchmarkId::new("string_match_indices", i),
            &string,
            |b, s| {
                b.iter(|| string_match_indices(black_box(s)));
            },
        );

        group.bench_with_input(BenchmarkId::new("chars_position", i), &string, |b, s| {
            b.iter(|| chars_position(black_box(s)));
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
