use core::cmp::Ordering;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::Diagnostic;

mod concise;
mod markdownlint;
mod mdl;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Concise,
    Mdl,
    Markdownlint,
}

impl Format {
    #[inline]
    #[must_use]
    pub fn sorter(&self) -> fn(a: &Diagnostic, b: &Diagnostic) -> Ordering {
        match self {
            Self::Concise => |a, b| Concise::new(a).cmp(&Concise::new(b)),
            Self::Mdl => |a, b| Mdl::new(a).cmp(&Mdl::new(b)),
            Self::Markdownlint => |a, b| Markdownlint::new(a).cmp(&Markdownlint::new(b)),
        }
    }
}

pub use concise::Concise;
pub use markdownlint::Markdownlint;
pub use mdl::Mdl;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use comrak::nodes::Sourcepos;

    use crate::rule::RuleLike as _;
    use crate::rule::{MD001, MD010};

    use super::*;

    #[allow(clippy::similar_names)]
    fn violations() -> Vec<Diagnostic> {
        let path1 = Path::new("foo.md").to_path_buf();
        let path2 = Path::new("bar.md").to_path_buf();
        let md001 = MD001::new();
        let md010 = MD010::new();
        let position0 = Sourcepos::from((1, 1, 1, 1));
        let position1 = Sourcepos::from((1, 2, 1, 2));
        let position2 = Sourcepos::from((2, 1, 2, 1));
        let violation1 = md001.to_violation(path1.clone(), position0);
        let violation2 = md001.to_violation(path1.clone(), position1);
        let violation3 = md001.to_violation(path1.clone(), position2);
        let violation4 = md001.to_violation(path2.clone(), position0);
        let violation5 = md001.to_violation(path2.clone(), position1);
        let violation6 = md001.to_violation(path2.clone(), position2);
        let violation7 = md010.to_violation(path1.clone(), position0);
        let violation8 = md010.to_violation(path1.clone(), position1);
        let violation9 = md010.to_violation(path1, position2);
        let violation10 = md010.to_violation(path2.clone(), position0);
        let violation11 = md010.to_violation(path2.clone(), position1);
        let violation12 = md010.to_violation(path2, position2);
        vec![
            Diagnostic::Violation(violation1),
            Diagnostic::Violation(violation2),
            Diagnostic::Violation(violation3),
            Diagnostic::Violation(violation4),
            Diagnostic::Violation(violation5),
            Diagnostic::Violation(violation6),
            Diagnostic::Violation(violation7),
            Diagnostic::Violation(violation8),
            Diagnostic::Violation(violation9),
            Diagnostic::Violation(violation10),
            Diagnostic::Violation(violation11),
            Diagnostic::Violation(violation12),
        ]
    }

    #[test]
    fn sorter_concise() {
        let violations = violations();
        let mut actual = violations.clone();
        actual.sort_by(Format::Concise.sorter());
        let mut outputs: Vec<_> = violations.iter().map(Concise::new).collect();
        outputs.sort();
        let expected: Vec<_> = outputs.iter().map(|o| o.diagnostic().clone()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn sorter_mdl() {
        let violations = violations();
        let mut actual = violations.clone();
        actual.sort_by(Format::Mdl.sorter());
        let mut outputs: Vec<_> = violations.iter().map(Mdl::new).collect();
        outputs.sort();
        let expected: Vec<_> = outputs.iter().map(|o| o.diagnostic().clone()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn sorter_markdownlint() {
        let violations = violations();
        let mut actual = violations.clone();
        actual.sort_by(Format::Markdownlint.sorter());
        let mut outputs: Vec<_> = violations.iter().map(Markdownlint::new).collect();
        outputs.sort();
        let expected: Vec<_> = outputs.iter().map(|o| o.diagnostic().clone()).collect();
        assert_eq!(actual, expected);
    }
}
