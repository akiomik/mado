use serde::Deserialize;

use crate::rule;
use crate::rule::md003::HeadingStyle;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
#[allow(clippy::exhaustive_structs)]
pub struct MD003 {
    pub style: HeadingStyle,
}

impl Default for MD003 {
    #[inline]
    fn default() -> Self {
        Self {
            #[allow(clippy::use_self)]
            style: rule::MD003::DEFAULT_HEADING_STYLE,
        }
    }
}

#[allow(clippy::use_self)]
impl From<&MD003> for rule::MD003 {
    #[inline]
    fn from(config: &MD003) -> rule::MD003 {
        rule::MD003::new(config.style.clone())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn from_for_rule_md003() {
        let style = HeadingStyle::SetextWithAtx;
        let config = MD003 {
            style: style.clone(),
        };
        let expected = rule::MD003::new(style);
        assert_eq!(rule::MD003::from(&config), expected);
    }
}
