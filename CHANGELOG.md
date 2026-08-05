# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to
[Semantic Versioning].

Entries describe changes a user of `mado` can observe: rules, the command line,
the configuration file, and the ways mado is distributed. Dependency updates,
CI, packaging internals, refactors and tests are left to the commit history —
except for the Markdown parser, whose version determines how documents are
parsed and therefore what gets reported.

## [Unreleased]

### Fixed

- MD030: derive the marker width from the item's own ordinal, so ordered items
  numbered 10 or higher are no longer reported when a single space already
  follows the marker (#363)
- MD004, MD005, MD029 and MD030: descend into containers other than lists, so
  lists nested inside a blockquote are checked (#368)
- MD007: measure indentation from the end of the blockquote prefix instead of
  the start of the line (#369)

## [0.3.1] - 2026-07-22

### Added

- Support for use as a [pre-commit] hook (#162)

### Changed

- Print errors to stderr instead of stdout (#247)
- Update the Markdown parser (`comrak`) from 0.38.0 to 0.54.0 (#161, #184,
  #192, #244, #337)

### Fixed

- Normalize a leading `./` before matching `--exclude` patterns (#344)

## [0.3.0] - 2025-04-12

### Added

- Rule tags, so groups of rules can be selected at once (#92)
- Shell completion generation (#125)

### Changed

- **Breaking:** update the Markdown parser (`comrak`) from 0.35.0 to 0.38.0,
  which changes how some documents are parsed (#139, #148)
- Require a subcommand, so running `mado` without one prints a usage error
  instead of exiting silently (#149)

## [0.2.2] - 2025-02-16

### Added

- `sublist` style for `[lint.md004]` (#107)
- `respect-gitignore` option for `[lint]` (#109)
- `exclude` option for `[lint]` (#115)

### Changed

- Speed up file discovery by filtering on file type while walking (#113)

## [0.2.1] - 2025-02-04

### Added

- `allow-different-nesting` option for `[lint.md024]` (#104)

## [0.2.0] - 2025-01-30

### Added

- Read from stdin in `check` (#89)
- JSON schema for the configuration file (#88)

### Changed

- **Breaking:** rename the config keys in `[lint.md030]` (#86)
- **Breaking:** change the style format for `[lint.md035]` (#91)
- Update the Markdown parser (`comrak`) from 0.33.0 to 0.35.0 (#95)

### Fixed

- `check` no longer fails on empty stdin (#96)

## [0.1.5] - 2025-01-22

### Added

- winget package (#74)
- `--quiet` flag (#78)

### Fixed

- Respect the configuration file when `--quiet` is given (#80)

## [0.1.4] - 2025-01-17

### Added

- Homebrew formula (#51, #52, #54, #55, #56, #57, #62)
- Scoop manifest (#53, #58)
- Nix flake (#60, #61)

## [0.1.3] - 2025-01-13

### Changed

- Update the Markdown parser (`comrak`) from 0.32.0 to 0.33.0 (#24)

## [0.1.2] - 2025-01-05

### Fixed

- MD013 now honours `code-blocks = false` and `tables = false`, and skips a
  whole table rather than only its rows (#14)

## [0.1.1] - 2025-01-05

### Added

- GitHub Action (#7, #8)
- `code-blocks` and `tables` options for MD013 (#9)

### Fixed

- Loading of the global configuration file (#10)

## [0.1.0] - 2024-12-31

### Added

- Initial release

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
[pre-commit]: https://pre-commit.com/
[Unreleased]: https://github.com/akiomik/mado/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/akiomik/mado/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/akiomik/mado/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/akiomik/mado/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/akiomik/mado/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/akiomik/mado/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/akiomik/mado/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/akiomik/mado/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/akiomik/mado/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/akiomik/mado/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/akiomik/mado/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/akiomik/mado/releases/tag/v0.1.0
