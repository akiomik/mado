//! `mado check` over a tree that has lost its Git metadata -- a source
//! archive, a Docker context copied without `.git` -- against `mado check`
//! over a clone of the same tree. The two reach their ignore files by
//! different routes: the walker finds them for a clone, and mado hands them
//! back where there is no repository to find them by. What they must never do
//! is disagree in the direction that hides something: a tree without the
//! metadata dropping a file its clone reports.
//!
//! The trees here are laid out by pairing rather than written out one by one,
//! because the shapes that have broken this are the ones nobody thought to
//! write: an ignore file two levels up taking back what one below it excludes,
//! a pattern anchored to a directory the walk names another way.

extern crate alloc;

use alloc::collections::BTreeSet;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, remove_file, write};
use std::path::Path;

use assert_cmd::Command;
use assert_cmd::cargo_bin;
use miette::{IntoDiagnostic as _, Result, miette};
use tempfile::tempdir;

/// Directories the trees are built out of.
const DIRS: &[&str] = &["", "docs", "docs/sub", "d1", "d1/docs"];

/// Where the ignore file under the boundary goes.
const HOLDER: &str = "docs";

/// The two kinds, which the walker ranks against each other by kind rather
/// than by where they sit.
const KINDS: &[&str] = &[".gitignore", ".ignore"];

/// Lines to fill an ignore file out with, beside the one that decides the
/// shape: anchored and not, naming a directory and naming a file.
const LINES: &[&str] = &[
    "a.md",
    "/docs/a.md",
    "/d1/",
    "docs/",
    "*.md",
    "/docs/sub/a.md",
    "**/a.md",
];

/// The names the walk is pointed at, and whether each leads where it reads.
/// The ones that do not are their own boundary, which reports more; the ones
/// that do have nothing to excuse a difference.
const NAMES: &[(&str, bool)] = &[(".", true), ("docs", true), ("docs/.", false)];

const CONFIGS: &[Option<&str>] = &[
    None,
    Some("[lint]\nrespect-gitignore = false\n"),
    Some("[lint]\nrespect-ignore = false\n"),
];

/// A generator whose trees do not move when a dependency does, so that a case
/// named by a failure goes on naming the same tree.
struct Seeded(u64);

impl Seeded {
    #[expect(
        clippy::integer_division_remainder_used,
        reason = "picking one of a list out of a number is what a remainder is for"
    )]
    const fn below(&mut self, end: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);

        ((self.0 >> 33) % end as u64) as usize
    }

    const fn one_in(&mut self, chance: usize) -> bool {
        self.below(chance) == 0
    }
}

/// One ignore file at the boundary and one under it, which is the pair the
/// walk and mado reach by different routes. `above` and `below` name their
/// kinds, and `takes_back` has the one at the boundary take `ignored.md` back
/// from the one under it -- the shape where handing a file back ranks it under
/// what the walk finds, and drops what a clone keeps.
struct Shape {
    above: &'static str,
    below: &'static str,
    takes_back: bool,
}

fn shapes() -> Vec<Shape> {
    let mut all = vec![];
    for above in KINDS {
        for below in KINDS {
            for takes_back in [false, true] {
                all.push(Shape {
                    above,
                    below,
                    takes_back,
                });
            }
        }
    }

    all
}

/// A tree of Markdown under `at`, carrying `shape`'s pair of ignore files.
fn write_tree(at: &Path, shape: &Shape, seed: u64) -> Result<()> {
    let mut rng = Seeded(seed);
    for dir in DIRS {
        create_dir_all(at.join(dir)).into_diagnostic()?;
        for name in ["a.md", "b.md"] {
            let text = if rng.one_in(4) { "# Fine\n" } else { "#Hello." };
            write(at.join(dir).join(name), text).into_diagnostic()?;
        }
        // The one the pair of ignore files disagrees about always has
        // something to report, so that a case cannot go quiet by chance.
        write(at.join(dir).join("ignored.md"), "#Hello.").into_diagnostic()?;
    }

    // The filler goes in first and the line that decides the shape last: an
    // ignore file is read to the end, and the last line to match is the one
    // that says what happens.
    let mut above = String::new();
    let mut below = String::new();
    for lines in [&mut above, &mut below] {
        for _ in 0..=rng.below(2) {
            lines.push_str(LINES[rng.below(LINES.len())]);
            lines.push('\n');
        }
    }
    if shape.takes_back {
        above.push_str("!ignored.md\n");
    }
    below.push_str("ignored.md\n");

    write(at.join(shape.above), above).into_diagnostic()?;
    write(at.join(HOLDER).join(shape.below), below).into_diagnostic()
}

/// The files `mado check name` reports on, named the way it named them.
fn reported(at: &Path, name: &str) -> Result<BTreeSet<String>> {
    let output = Command::new(cargo_bin!("mado"))
        .current_dir(at)
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .args(["check", name])
        .output()
        .into_diagnostic()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split(':').next())
        .filter(|head| Path::new(head).extension().is_some_and(|kind| kind == "md"))
        .map(ToOwned::to_owned)
        .collect())
}

/// Fails where an `.ignore` above `root` takes a path back with a `!` line.
/// mado reads every parent directory looking for one, so a file like that on
/// the machine running the tests changes what these trees report, and the
/// failure would be nothing to do with the tree under test.
fn nothing_above_takes_a_path_back(root: &Path) -> Result<()> {
    let at = root.canonicalize().into_diagnostic()?;
    for over in at.ancestors().skip(1) {
        let file = over.join(".ignore");
        let takes_back =
            read_to_string(&file).is_ok_and(|text| text.lines().any(|line| line.starts_with('!')));
        if takes_back {
            return Err(miette!(
                "{} takes a path back, and these tests need no such file over them",
                file.display()
            ));
        }
    }

    Ok(())
}

#[test]
fn an_archive_reports_everything_a_clone_of_it_does() -> Result<()> {
    for (at, shape) in shapes().iter().enumerate() {
        let tmp_dir = tempdir().into_diagnostic()?;
        nothing_above_takes_a_path_back(tmp_dir.path())?;
        let proj = tmp_dir.path().join("proj");
        write_tree(&proj, shape, at as u64)?;

        for config in CONFIGS {
            let settings = proj.join("mado.toml");
            match *config {
                Some(text) => write(&settings, text).into_diagnostic()?,
                None if settings.exists() => remove_file(&settings).into_diagnostic()?,
                None => {}
            }

            for &(name, leads_where_it_reads) in NAMES {
                let git = proj.join(".git");
                let archive = reported(&proj, name)?;
                create_dir_all(&git).into_diagnostic()?;
                let clone = reported(&proj, name)?;
                remove_dir_all(&git).into_diagnostic()?;

                let case = format!(
                    "{} over {HOLDER}/{}{}, {config:?}, {name:?}",
                    shape.above,
                    shape.below,
                    if shape.takes_back {
                        ", taking back"
                    } else {
                        ""
                    },
                );
                let dropped: Vec<&String> = clone.difference(&archive).collect();
                assert!(
                    dropped.is_empty(),
                    "{case}: the archive dropped {dropped:?}"
                );

                // Nothing excuses a difference where both files are
                // `.gitignore` and the name leads where it reads.
                let all_gitignore = shape.above == ".gitignore" && shape.below == ".gitignore";
                if all_gitignore && leads_where_it_reads {
                    let extra: Vec<&String> = archive.difference(&clone).collect();
                    assert!(extra.is_empty(), "{case}: the archive reported {extra:?}");
                }
            }
        }

        tmp_dir.close().into_diagnostic()?;
    }

    Ok(())
}
