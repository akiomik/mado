# Changelog

## 0.3.0 (2025-04-12)

### ⚠️ BREAKING CHANGES

* build(deps): bump comrak from 0.36.0 to 0.38.0 by @dependabot in #148

### Features

* feat: tags by @akiomik in #92
* feat: shell completion by @akiomik in #125

### Other Changes

* chore: bump packages to v0.2.2 by @akiomik in #121
* chore: update README.md by @akiomik in #122
* refactor: add enum Tag by @akiomik in #123
* chore: enable clippy::integer_division_remainder_used by @akiomik in #124
* build(deps): bump tempfile from 3.16.0 to 3.17.0 by @dependabot in #127
* build(deps): bump clap from 4.5.28 to 4.5.29 by @dependabot in #126
* build(deps): bump serde from 1.0.217 to 1.0.219 by @dependabot in #135
* build(deps): bump etcetera from 0.8.0 to 0.10.0 by @dependabot in #134
* build(deps): bump tempfile from 3.17.0 to 3.18.0 by @dependabot in #133
* build(deps): bump clap from 4.5.29 to 4.5.31 by @dependabot in #128
* build(deps): bump globset from 0.4.15 to 0.4.16 by @dependabot in #131
* build(deps): bump comrak from 0.35.0 to 0.36.0 by @dependabot in #139
* build(deps): bump scraper from 0.22.0 to 0.23.1 by @dependabot in #138
* build(deps): bump indoc from 2.0.5 to 2.0.6 by @dependabot in #137
* build(deps): bump tempfile from 3.18.0 to 3.19.0 by @dependabot in #136
* build(deps): bump mimalloc from 0.1.43 to 0.1.45 by @dependabot in #147
* build(deps): bump clap from 4.5.32 to 4.5.35 by @dependabot in #146
* build(deps): bump tempfile from 3.19.0 to 3.19.1 by @dependabot in #145
* refactor: make subcommand required by @akiomik in #149
* build(deps): bump comrak from 0.36.0 to 0.38.0 by @dependabot in #148

## 0.2.2 (2025-02-16)

### Features

* feat: add sublist to style in [lint.md004] by @akiomik in #107
* feat: add respect-gitignore to [lint] by @akiomik in #109
* Feat exclude by @akiomik in #115

### Other Changes

* chore: enable MD024 in mado.toml by @akiomik in #106
* chore: fix `just cov` by @akiomik in #108
* build(deps): bump clap from 4.5.27 to 4.5.28 by @dependabot in #112
* build(deps): bump rustc-hash from 2.1.0 to 2.1.1 by @dependabot in #111
* build(deps): bump toml from 0.8.19 to 0.8.20 by @dependabot in #110
* perf: use ignore::Types by @akiomik in #113
* refactor: add with_tmp_file helper by @akiomik in #114
* test: use pretty_assertions by @akiomik in #116
* test: disable allow-unwrap-in-tests by @akiomik in #117
* Test indoc by @akiomik in #118
* test: refactor for WalkParallelBuilder by @akiomik in #119

## 0.2.1 (2025-02-04)

### Features

* feat: add allow-different-nesting to [lint.md024] by @akiomik in #104

### Other Changes

* chore: bump packages to v0.2.0 by @akiomik in #100
* use rustc-hash by @akiomik in #101
* build(deps): bump miette from 7.4.0 to 7.5.0 by @dependabot in #102
* build(deps): bump tempfile from 3.15.0 to 3.16.0 by @dependabot in #103

## 0.2.0 (2025-01-30)

### ⚠️ BREAKING CHANGES

* fix!: rename config keys in [lint.md030] by @akiomik in #86
* feat!: change style format for [lint.md035] by @akiomik in #91

### Features

* feat: add stdin support to check by @akiomik in #89
* feat: json schema support by @akiomik in #88

### Bug Fixes

* fix: check command with empty stdin by @akiomik in #96

### Other Changes

* chore: update packages to 0.1.5 by @akiomik in #85
* chore: add update-winget to justfile by @akiomik in #84
* chore: add breaking change to .github/release.yml by @akiomik in #87
* Taplo ci by @akiomik in #90
* build(deps): bump clap from 4.5.26 to 4.5.27 by @dependabot in #94
* build(deps): bump comrak from 0.33.0 to 0.35.0 by @dependabot in #95
* build(deps): bump rand from 0.8.5 to 0.9.0 by @dependabot in #93

## 0.1.5 (2025-01-22)

### Features

* Winget by @akiomik in #74
* feat: add --quiet flag by @hougesen in #78
* feat: add Serialize for Config by @akiomik in #81

### Bug Fixes

* fix: respect config with --quiet by @akiomik in #80

### Other Changes

* Run justfile --fmt by @akiomik in #68
* Update packages to v0.1.4 by @akiomik in #67
* Remove .cargo/config.toml by @akiomik in #69
* Use rust 1.84 by @akiomik in #70
* Nursery by @akiomik in #71
* Update README.md by @akiomik in #72
* Fix use_self by @akiomik in #73
* Add test for MarkdownLintVisitorFactory#build by @akiomik in #75
* Add test for ParallelLintRunner#run by @akiomik in #76
* ci: update .github/release.yml by @akiomik in #79

## 0.1.4 (2025-01-17)

* Minor improvements (#41, #42, #45, #46, #49)
* Bump colored from 2.2.0 to 3.0.0 (#43)
* Bump clap from 4.5.23 to 4.5.26 (#44)
* Add fuzz testing (#47)
* Update README.md (#48, #50)
* Add Homebrew support (#51, #52, #54, #55, #56, #57, #62)
* Add Scoop support (#53, #58)
* Add justfile (#59)
* Add Nix support (#60, #61)
* Add `.github/release.yml` (#63, #65)

## 0.1.3 (2025-01-13)

* Update project `mado.toml` (#13)
* Minor improvements (#19, #20, #23, #26, #29, #31, #32, #35, #39)
* Add tests (#21, #22, #33, #34, #36, #37, #38)
* Bump comrak from 0.32.0 to 0.33.0 (#24)
* Fix benchmark results (#25)
* Improve CI (#27, #30)
* Update README.md (#28)

## 0.1.2 (2025-01-05)

* Update `README.md` (#12, #17)
* Fix MD013 (#14)
* Fix `Cargo.toml` (#15)
* Add `Document#lines` (#16)

## 0.1.1 (2025-01-05)

* Add github action support (#7, #8)
* Add `code_blocks` and `tables` options to MD013 (#9)
* Fix global configuration loading (#10)

## 0.1.0 (2024-12-31)

* Initial release!
