use core::hint::black_box;
use std::sync::LazyLock;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::distr::SampleString as _;
use rand::distr::StandardUniform;
use regex::Regex;

static RE: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(" +$").unwrap()
});

fn regex_captures(s: &str) -> Option<(usize, usize)> {
    let mut locs = RE.capture_locations();
    RE.captures_read(&mut locs, s);
    if let Some((start_column, end_column)) = locs.get(0) {
        return Some((start_column + 1, end_column));
    }

    None
}

fn string_trim_end_matches(s: &str) -> Option<(usize, usize)> {
    let trimmed = s.trim_end_matches(' ');
    if s != trimmed {
        return Some((trimmed.len() + 1, s.len()));
    }

    None
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("trailing_space_detection");

    for i in 1..=3 {
        let string = StandardUniform.sample_string(&mut rand::rng(), (i as usize).pow(i) * 100);

        group.bench_with_input(BenchmarkId::new("regex_captures", i), &string, |b, s| {
            b.iter(|| regex_captures(black_box(s)));
        });

        group.bench_with_input(
            BenchmarkId::new("string_trim_end_matches", i),
            &string,
            |b, s| b.iter(|| string_trim_end_matches(black_box(s))),
        );
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
