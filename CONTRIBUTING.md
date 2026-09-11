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

`just lint`, and so `just`, needs a C++ toolchain on the machine: it lints the
whole workspace, which reaches the fuzz target, and `libfuzzer-sys` compiles a
vendored libFuzzer before anything of mado's is looked at. A machine without one
fails inside that build rather than in anything the change touched. `just test`
does not need a C++ one, for the `false`s in `fuzz/Cargo.toml`, and neither does
CI's coverage job. CI's Clippy job installs `g++` on a runner that has none.

## Changing a configuration key's shape

`mado.toml` here is read by two mado binaries: the one built from the branch,
and the newest published release, which the `Download` job in
`.github/workflows/ci-action.yml` runs over this checkout through the action.
That job is also the Markdown quality gate for this repository — nothing else
lints these documents — so it cannot be pointed at a fixture instead.

Adding a key is safe, and so is removing or renaming one: the release ignores
keys it does not know and gives absent ones their defaults. What it cannot read
is an existing key whose type has changed, or a value it has never heard of,
such as a variant added to an enum. It fails to load the file, and the job fails
before it lints anything.

A change of that kind therefore lands in two steps:

1. In the pull request that changes the key, take it out of `mado.toml`, and
   open an issue for putting it back. Nothing fails while the key is missing, so
   the second step has to be somewhere that is looked at rather than remembered.
   Say in the changelog what a configuration carrying the old spelling has to do.
1. After the release that carries the change is published, restore the key with
   its new spelling. Step 4 of [Releasing](#releasing) is where the rest of the
   after-the-release work lives, and this belongs in the same pull request.

The action's `version` input met the same constraint from the other side: it
defaults to the release it was published with, which does not exist yet at the
moment of a version bump, so the job names the newest published release instead.

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
1. Bump the version in the files the tagged commit has to be right about:
   `version` in `Cargo.toml` and `Cargo.lock`, `DEFAULT_VERSION` in
   `action/entrypoint.sh`, and the `akiomik/mado@vx.y.z` pins in `README.md`.
   The GitHub Action downloads the release `entrypoint.sh` names, so a tag that
   leaves it behind runs the previous version's binary. CI checks that these
   agree with `Cargo.toml`, so forgetting one fails before the tag.
1. Tag the merged commit `vx.y.z` and push the tag. That starts CD.
1. Once CD has published the release, restore anything
   [a configuration change](#changing-a-configuration-keys-shape) had to take
   out of `mado.toml`, set `version` / `prev_version` in the `justfile`, refresh
   the package manifests with `just update-homebrew`, `just update-scoop`,
   `just update-winget` and `just update-flake`, and open a pull request for
   them. Every one of these recipes downloads assets of the
   release being packaged, so none of them can run any earlier. This is also why
   `flake.nix` and the manifests under `pkg/` still name the previous version at
   the moment the tag is pushed, and why `nix run github:akiomik/mado/vx.y.z`
   gets the release before it. `just update-winget` also stamps `ReleaseDate` —
   the date WinGet shows the installer as released on — from the changelog
   heading for the version, and stops rather than leave the previous release's
   date behind if step 1 left that section undated.

CD builds the binaries for every platform and publishes one release once all of
them are packaged. Its body is that version's changelog section, extracted by
`scripts/release/extract-changelog.sh`.

Only a `vx.y.z` tag starts CD, and a tag without the `v` is ignored with no run
to look at. It also stops before building anything if the tag disagrees with any
of the versions in step 2, or if the changelog has no section for it.

Re-running CD for a tag that already has a release fails: the publish action
refuses to add to one, whether or not it could. Delete the release first if you
need to build the tag again.

CI checks that the section for the version in `Cargo.toml` exists, so a bump
without a changelog section fails before it reaches the tag.
