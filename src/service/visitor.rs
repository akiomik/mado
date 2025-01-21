use core::result::Result;
use std::{path::Path, sync::mpsc::SyncSender};

use comrak::Arena;
use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkState};

use super::Linter;
use crate::{
    config::Config,
    diagnostic::{IoError, LintError},
    Diagnostic, Document,
};

pub struct MarkdownLintVisitor {
    linter: Linter,
    tx: SyncSender<Vec<Diagnostic>>,
}

impl MarkdownLintVisitor {
    #[inline]
    #[must_use]
    pub const fn new(linter: Linter, tx: SyncSender<Vec<Diagnostic>>) -> Self {
        Self { linter, tx }
    }

    fn visit_inner(&self, either_entry: Result<DirEntry, Error>) -> Option<Vec<Diagnostic>> {
        match either_entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() && path.extension() == Some("md".as_ref()) {
                    let arena = Arena::new();
                    match Document::open(&arena, path) {
                        Ok(doc) => match self.linter.check(&doc) {
                            Ok(violations) => {
                                let violations: Vec<_> = violations
                                    .iter()
                                    .map(|violation| Diagnostic::Violation(violation.to_owned()))
                                    .collect();
                                Some(violations)
                            }
                            Err(err) => {
                                let lint_error =
                                    LintError::new(path.to_path_buf(), err.to_string());
                                Some(vec![Diagnostic::LintError(lint_error)])
                            }
                        },
                        Err(err) => {
                            let io_error = IoError::new(path.to_path_buf(), err.to_string());
                            Some(vec![Diagnostic::IoError(io_error)])
                        }
                    }
                } else {
                    None
                }
            }
            Err(ignore_err) => match ignore_err {
                Error::WithPath { path, ref err } => {
                    let io_error = IoError::new(path, err.to_string());
                    Some(vec![Diagnostic::IoError(io_error)])
                }
                err => {
                    // TODO: Use proper path
                    let io_error = IoError::new(Path::new(".").to_path_buf(), err.to_string());
                    Some(vec![Diagnostic::IoError(io_error)])
                }
            },
        }
    }
}

impl ParallelVisitor for MarkdownLintVisitor {
    #[inline]
    fn visit(&mut self, either_entry: Result<DirEntry, Error>) -> WalkState {
        if let Some(diagnostics) = self.visit_inner(either_entry) {
            if !diagnostics.is_empty() {
                if let Err(err) = self.tx.send(diagnostics) {
                    // TODO: Handle error
                    println!("{err}");
                }
            }
        }

        WalkState::Continue
    }
}

pub struct MarkdownLintVisitorFactory {
    config: Config,
    tx: SyncSender<Vec<Diagnostic>>,
}

impl MarkdownLintVisitorFactory {
    #[inline]
    #[must_use]
    pub const fn new(config: Config, tx: SyncSender<Vec<Diagnostic>>) -> Self {
        Self { config, tx }
    }
}

impl<'s> ParallelVisitorBuilder<'s> for MarkdownLintVisitorFactory {
    #[inline]
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        let linter = Linter::from(&self.config);
        Box::new(MarkdownLintVisitor::new(linter, self.tx.clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File, Permissions},
        io::Write as _,
        os::unix::fs::PermissionsExt as _,
        sync::mpsc,
        thread,
    };

    use ignore::Walk;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn markdown_lint_visitor_visit_inner() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<Diagnostic>>(0);
        let linter = Linter::new(vec![]);
        let visitor = MarkdownLintVisitor::new(linter, tx);

        for either_entry in Walk::new(".") {
            let actual = visitor.visit_inner(either_entry.clone());
            let entry = either_entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension() == Some("md".as_ref()) {
                assert_eq!(actual, Some(vec![]));
            } else {
                assert_eq!(actual, None);
            }
        }
    }

    #[test]
    fn markdown_lint_visitor_visit_inner_invalid_markdown() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<Diagnostic>>(0);
        let linter = Linter::new(vec![]);
        let visitor = MarkdownLintVisitor::new(linter, tx);

        // Create a binary file as Markdown
        let tmp_dir = tempdir().unwrap();
        let path = tmp_dir.path().join("invalid.md");
        let mut tmp_file = File::create(path.clone()).unwrap();
        tmp_file.write_all(b"\xf6").unwrap();

        for either_entry in Walk::new(path.clone()) {
            let actual = visitor.visit_inner(either_entry.clone());
            let error = IoError::new(
                path.clone(),
                "stream did not contain valid UTF-8".to_owned(),
            );
            let diagnostic = Diagnostic::IoError(error);
            assert_eq!(actual, Some(vec![diagnostic]));
        }

        tmp_dir.close().unwrap();
    }

    #[test]
    fn markdown_lint_visitor_visit_inner_file_not_exist() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<Diagnostic>>(0);
        let linter = Linter::new(vec![]);
        let visitor = MarkdownLintVisitor::new(linter, tx);

        let tmp_dir = tempdir().unwrap();
        let path = tmp_dir.path().join("not-exist.md");

        for either_entry in Walk::new(path.clone()) {
            // NOTE: Walk and WalkParallel look to show different error messages
            let actual = visitor.visit_inner(either_entry.clone());
            let error = IoError::new(
                path.clone(),
                format!(
                    "IO error for operation on {}: No such file or directory (os error 2)",
                    path.to_str().unwrap()
                ),
            );
            let diagnostic = Diagnostic::IoError(error);
            assert_eq!(actual, Some(vec![diagnostic]));
        }

        tmp_dir.close().unwrap();
    }

    #[test]
    fn markdown_lint_visitor_visit_inner_permission_denined() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<Diagnostic>>(0);
        let linter = Linter::new(vec![]);
        let visitor = MarkdownLintVisitor::new(linter, tx);

        // Create unreadable file as Markdown
        let tmp_dir = tempdir().unwrap();
        let path = tmp_dir.path().join("unreadable.md");
        File::create(path.clone()).unwrap();
        fs::set_permissions(path.clone(), Permissions::from_mode(0o000)).unwrap();

        for either_entry in Walk::new(path.clone()) {
            let actual = visitor.visit_inner(either_entry.clone());
            let error = IoError::new(path.clone(), "Permission denied (os error 13)".to_owned());
            let diagnostic = Diagnostic::IoError(error);
            assert_eq!(actual, Some(vec![diagnostic]));
        }

        tmp_dir.close().unwrap();
    }

    #[test]
    fn markdown_lint_visitor_factory_build() {
        let mut config = Config::default();
        config.lint.rules = vec![];

        let (tx, rx) = mpsc::sync_channel::<Vec<Diagnostic>>(0);
        let mut factory = MarkdownLintVisitorFactory::new(config, tx);

        for entry in Walk::new(".") {
            let mut visitor = factory.build();

            thread::spawn(move || {
                let state = visitor.visit(entry);
                assert_eq!(state, WalkState::Continue);
            });
        }

        drop(factory);
        assert!(rx.recv().is_err()); // Because rx has not received any messages
    }
}
