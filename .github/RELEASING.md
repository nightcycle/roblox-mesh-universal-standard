# Releases

The repository contains two products at matching versions: the Rust `rmus`
executable and `nightcycle/roblox-mesh-universal-standard` Wally package.

1. Update Cargo.toml and mesh-runtime/wally.toml together and regenerate Cargo.lock.
2. Run CI checks and the Studio integration suite. Rebuild demo exports when the
   format changes. Commit the source, lockfile, fixtures and generated demo assets.
3. Push an annotated `vX.Y.Z` tag to the checked commit. The Release workflow waits
   for CI, builds Windows x64, Linux x64, macOS x64 and macOS ARM64 ZIPs, then
   publishes those archives, the Wally ZIP, package RBXM, demo RBXL and SHA256SUMS.
   Executables are at the root of each platform-named ZIP for Rokit discovery.
4. Configure repository secret `WALLY_AUTH` with either the raw token stored by
   `wally login` for `https://api.wally.run/` or the complete auth.toml file.
   The account must be allowed to publish in the nightcycle scope. The workflow
   validates and writes the TOML config without logging the token, then removes
   the credential file after publishing, including when publishing fails.
   Run **Publish Wally** manually with the published version tag.
5. Verify installation in a fresh consumer project:
   `rokit add nightcycle/roblox-mesh-universal-standard@X.Y.Z`, `rmus --version`,
   then `wally install` with the documented dependency.

The workflows do not publish merely by building locally. A tag triggers the
GitHub release; the Wally workflow is a separate explicit publication step.
Both workflows build or publish the tagged source. A retry of an old tag still
uses the code at that tag, so release tags must contain the relevant fixes.

To retry the GitHub release, open **Actions → Release → Run workflow**, select
the branch containing the updated workflow (normally `main`), and enter the
existing version tag, such as `v0.1.0`. The workflow resolves that tag once and
runs CI and all builds against the same commit. It creates a missing release,
or replaces assets with matching names on an existing release and publishes it
if it was still a draft. Runs for the same tag are serialized.

The CLI equivalent is:

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0
```
