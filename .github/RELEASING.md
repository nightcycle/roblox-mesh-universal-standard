# Releases

The repository contains two products at matching versions: the Rust `rmus`
executable and `nightcycle/roblox-mesh-universal-standard` Wally package.

1. Update Cargo.toml and mesh-runtime/wally.toml together and run `cargo check`
   to update the package entry in Cargo.lock. All three versions must match.
2. Run CI checks and the Studio integration suite. Rebuild demo exports when the
   format changes. Commit the source, lockfile, fixtures and generated demo assets.
3. Push an annotated `vX.Y.Z` tag to the checked commit. The Release workflow waits
   for CI, builds Windows x64, Linux x64, macOS x64 and macOS ARM64 ZIPs, then
   publishes those archives, the Wally ZIP, package RBXM, demo RBXL and SHA256SUMS.
   Executables are at the root of each platform-named ZIP for Rokit discovery.
   The preparation job displays the exact tag commit and validates versions
   before CI. Publishing uploads all assets to a draft before making it public.
4. Configure repository secret `WALLY_AUTH` with either the raw token stored by
   `wally login` for `https://api.wally.run/` or the complete auth.toml file.
   The account must be allowed to publish in the nightcycle scope. The workflow
   validates and writes the TOML config without logging the token, then removes
   the credential file after publishing, including when publishing fails.
   Run **Publish Wally** manually from the branch containing the publishing fixes
   (normally `main`), with the published version tag as the `tag` input. Publishing
   scripts come from the selected branch's commit; the package is checked out
   separately from the tag into `release-source`. Older tags do not need to
   contain the newer publishing helpers.
5. Verify installation in a fresh consumer project:
   `rokit add nightcycle/roblox-mesh-universal-standard@X.Y.Z`, `rmus --version`,
   then `wally install` with the documented dependency.

The workflows do not publish merely by building locally. A tag triggers the
GitHub release; the Wally workflow is a separate explicit publication step.
Normal releases and Wally publishing use the tagged package source. Wally
publishing-script fixes come from the selected workflow branch and can be used
to retry an older tag.

To retry the GitHub release, open **Actions → Release → Run workflow**, select
the branch containing the updated workflow (normally `main`), and enter the
existing version tag, such as `v0.1.0`. The workflow resolves that tag once and
runs CI and all builds against the same commit. It creates a missing release,
or replaces assets on an unfinished draft and then publishes it. Already
published releases require the explicit repair option described below.
Runs for the same tag are serialized.

To repair the initial `0.1.0` release while keeping that version:

1. Keep Cargo.toml, Cargo.lock and mesh-runtime/wally.toml at `0.1.0`.
2. Commit and push the fixes, including this workflow, to `main`.
3. Open **Actions → Release → Run workflow**, choose `main`, enter `v0.1.0`,
   and enable **repair**.

Repair pins the selected branch's commit when the workflow starts. It validates
versions and runs CI and all builds against that commit. Only after they pass
does it move `v0.1.0` to that commit and replace the release assets. It refuses
immutable releases and aborts if another process moved the tag during the run.
If uploading fails after the tag moves, dispatch repair again to finish it.

Without repair, selecting `main` selects only the workflow implementation;
the source still comes from the existing tag. Repair deliberately changes the
source behind an existing version, so use it to recover this broken initial
release. Future code changes should normally receive a new version.

The equivalent repair command is:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0 -f repair=true
```

For a normal retry without moving the tag:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0
```

To retry Wally publishing with the current automation and an existing release:

```sh
gh workflow run wally.yml --ref main -f tag=v0.1.0
```
