# Contributing

Thanks for your interest in mado!

## Development

The toolchain is pinned by `rust-toolchain.toml`, so a rustup installation picks
up the right version on its own. [just](https://github.com/casey/just) runs the
same checks CI does:

```console
just        # fmt, test and lint
just test
just lint
```

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
A pull request that changes something a user can observe adds its entry under
`## [Unreleased]` in the same pull request, under `Added`, `Changed`, `Fixed`,
`Deprecated`, `Removed` or `Security`.

Observable means one of:

- what `mado check` reports — a new rule, a fixed false positive or negative, a
  changed default
- the command line or the configuration file
- how mado is installed or distributed
- the Markdown parser (`comrak`) version, which decides how documents are parsed
  and therefore what gets reported

Everything else is left to the commit history: dependency updates, CI, packaging
internals, refactors and tests. If a change has no entry, that is the answer to
"was this observable?", not an omission.

Write entries for someone reading the release notes, not for someone reading the
diff. Name the rule, say what changed about its behaviour, and reference the pull
request:

```markdown
- MD007: measure indentation from the end of the blockquote prefix instead of
  the start of the line (#369)
```

Mark a breaking change with a `**Breaking:**` prefix under `Changed`.

## Releasing

1. Rename `## [Unreleased]` to `## [x.y.z] - YYYY-MM-DD`, add a fresh empty
   `## [Unreleased]` above it, and update the link definitions at the bottom of
   the file.
1. Bump the version everywhere it is written down: `version` in `Cargo.toml`
   and `Cargo.lock`, `version` / `prev_version` in the `justfile`, `VERSION` in
   `action/entrypoint.sh`, and the `akiomik/mado@vx.y.z` pins in `README.md`.
   The GitHub Action downloads the release `entrypoint.sh` names, so a tag that
   leaves it behind runs the previous version's binary.
1. Tag the merged commit `vx.y.z` and push the tag. That starts CD.
1. Once CD has published the release, refresh the package manifests with
   `just update-homebrew`, `just update-scoop`, `just update-winget` and
   `just update-flake`, and open a pull request for them. Every one of these
   recipes downloads assets of the release being packaged, so none of them can
   run any earlier.

CD builds the binaries for every platform and publishes one release once all of
them are packaged. Its body is that version's changelog section, extracted by
`scripts/release/extract-changelog.sh`.

Only a `vx.y.z` tag starts CD, and a tag without the `v` is ignored with no run
to look at. It also stops before building anything if the tag disagrees with
`version` in `Cargo.toml`, or if the changelog has no section for it.

Re-running CD for a tag that already has a release fails: the publish action
refuses to add to one, whether or not it could. Delete the release first if you
need to build the tag again.

CI checks that the section for the version in `Cargo.toml` exists, so a bump
without a changelog section fails before it reaches the tag.
