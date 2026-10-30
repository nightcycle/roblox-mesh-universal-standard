# Roblox Mesh Universal Standard

RMUS converts a single mesh into portable, versioned JSON. The Rust executable
`rmus` extracts geometry; the strictly typed Wally package
`nightcycle/roblox-mesh-universal-standard` recreates and caches Roblox EditableMeshes.

## Export

```sh
cargo build --release --locked
rmus export test/pillar.rbxm --output pillar.json --children pillar.model.json

rmus export rbxassetid://6436440147 --output pillar.json
rmus export model.obj --output model.json --scale 1
rmus export model.fbx --output model.json --pretty
rmus validate pillar.json
rmus inspect test/grass.rbxm test/lilypad.rbxm test/pillar.rbxm
```

Use `target/release/rmus` (Windows: `target/release/rmus.exe`) until installed.
Run `rmus export --help` for all options.
Exports are raw, compact JSON by default. Add `--pretty` to indent both the mesh
JSON and optional `.model.json` sidecar. Rojo converts the mesh `.json` file into
a ModuleScript automatically.

Inputs:

| Input | Behavior |
| --- | --- |
| RBXM / RBXMX | Exactly one MeshPart or FileMesh SpecialMesh; resolves its mesh asset. Rejects multi-mesh selections, including repeated instances of one asset. |
| `rbxassetid://ID`, numeric ID, Roblox asset URL | Downloads a raw mesh or a single-mesh model using the same asset-delivery method as imposter-baker. |
| Roblox FileMesh | Versions 1.00/1.01, 2.00, 3.00/3.01, 4.00/4.01, 5.00, 6.00 and 7.00 (Draco). Uses the highest-detail render LOD. |
| OBJ | Polygon triangulation, negative indices, split normals, UVs, vertex colors. A single mesh object is required. |
| FBX | Binary and ASCII, transforms, split normals/colors/UVs, linear skinning, bone hierarchy, and Roblox `RootFaceJoint` + `FrameN` facial pose properties. |
| RMUS JSON | Validates and emits JSON, preserving metadata and extensions. |

OBJ/FBX axes are converted to Roblox's right-handed, Y-up coordinate system.
Source units are preserved; `--scale` explicitly converts them to studs.
FBX needs one animation take for facial poses, a mapped `Neutral` frame, and
bone-based facial animation. Vertex blend shapes, vertex-cache animation and
nonlinear skinning cannot be represented by EditableMesh and cause an error.
Ordinary animation clips are not mesh data and are not exported.

No texture images or material libraries are downloaded, decoded or baked.
Vertex RGBA is retained. UV coordinates are retained for structural fidelity.
The optional RBXM sidecar preserves existing texture asset references, including
SurfaceAppearances, without fetching the images.

Downloads are cached in `.rmus-cache/<id>.bin`. `--offline` disables network
access. For private/unavailable assets, provide authorized local copies using
`--asset-map map.json`:

```json
{ "rbxassetid://123": "meshes/123.mesh" }
```

Paths are relative to the map file. The tool does not read browser credentials.
`--metadata metadata.json` merges a developer-defined JSON object into the export.

## Hydrate in Roblox

Add the exported `.json` file to your Rojo project and require the Wally package.
Pass the ModuleScript created by Rojo to the runtime:

```luau
--!strict
local Mesh = require(game.ReplicatedStorage.Packages.MeshUniversal)
local dataModule = game.ReplicatedStorage.Assets.Pillar

local editable = Mesh.getAsync(dataModule) -- FixedSize = true by default; may yield
assert(editable == Mesh.getAsync(dataModule))

local part = Mesh.createMeshPartAsync(
    dataModule,
    game.ReplicatedStorage.Assets.PillarChildren
)
part.Anchored = true
part.Parent = workspace

part:Destroy()
Mesh.release(dataModule)
```

Decoded tables use an explicit developer-owned ID:

```luau
local editable = Mesh.getAsync(decodedJson, { Id = "terrain/chunk-04", FixedSize = false })
-- The same ID reuses the first mesh even if supplied through a different table.
local same = Mesh.getAsync(decodedJson, { Id = "terrain/chunk-04", FixedSize = false })
Mesh.release("terrain/chunk-04")
```

ModuleScripts are keyed by **Instance identity**. Requiring the same module through
multiple references shares one mesh. Different ModuleScript instances are different
keys. Strings and module keys occupy separate namespaces.

`getAsync` is an idempotent cache lookup, not reference counting. Each `createMeshPartAsync`
call creates a new caller-owned part sharing that cached mesh. Destroy all parts
using a mesh before `release`; eviction immediately destroys the shared mesh.
Do not call `Destroy` directly on a cached EditableMesh. Changes to a cached mesh
are visible to every caller. Reusing a key with a different `FixedSize` value is an
error; release it first or use another key.

Simultaneous requests share one in-flight hydration. Failed loads are evicted.
Releasing a key while it is loading cancels that request and prevents it from
repopulating the cache. Retry with the same key starts a fresh load.

Fixed-size hydration first constructs a temporary dynamic mesh, then calls
`CreateEditableMeshAsync(Content.fromObject(mesh), { FixedSize = true })` and
destroys the temporary object. This transiently needs memory for both meshes.
Roblox's allocation/budget errors propagate with cleanup.

## Child reconstruction

`--children pillar.model.json` writes a Rojo-compatible Configuration with a MeshPart named
`Template`. It retains part properties and the mesh's descendant hierarchy:
attachments, bones, SurfaceAppearances, values, attributes, tags, and other
serializable children. Geometry and engine identity fields are excluded.

`createMeshPartAsync(moduleOrTable, configuration?, options?)` clones the template,
applies the hydrated mesh, restores part appearance/size/transform, and remaps
internal Instance references. The input Configuration is reusable and remains
unchanged. References to objects outside the mesh subtree produce an export error
instead of being silently broken.

Without a Configuration, `createMeshPartAsync` restores the common properties stored in
`document.Part`; descendant instances and extra properties require the sidecar.
Reconstruction recreates render geometry and regenerates collision using the
recorded CollisionFidelity. Roblox does not expose the original cooked collision
blob through EditableMesh, so byte-identical collision data is not promised.
Source assets must remain accessible for retained texture references to display.

## Versioning and API

The schema is documented in [docs/format.md](docs/format.md), with a
[JSON Schema](schema/mesh-v1.schema.json). All array references are **1-based** and
are remapped to Roblox's opaque IDs during hydration.

Interface functions use camelCase and `.`. Instance methods use PascalCase and
`:`, such as `part:Destroy()`. Functions that can yield or do substantial work
carry an `Async` suffix; they return their documented values directly.
All defined data fields use PascalCase, including the raw JSON fields
`Format`, `Version.Major`, `Vertices`, `Faces`, `UVs`, and `Part`.
Regenerate exports from earlier development builds to use the updated field names.

- `getAsync(source, options?) -> EditableMesh`
- `createMeshPartAsync(source, configuration?, options?) -> MeshPart`
- `release(moduleOrId) -> boolean`, `clearAsync()`, `isCached(moduleOrId) -> boolean`
- `validateAsync(document) -> Document`
- `exportAsync(editableMesh) -> Document`: captures live geometry, split attributes,
  bones, weights, base FACS poses and two/three-action corrective targets.
- `registerVersion(major, migrate)`: register a future-major migration into v1.
- `registerExtension(name, hydrate)`: populate future mesh capabilities before
  fixed-size conversion. The callback receives `(mesh, payload, ids)`.

Unknown optional extensions are retained by the Rust exporter and ignored by the
runtime. Unknown required features fail explicitly. Extensions that must hydrate
belong in `RequiredFeatures` as well as `Extensions`.

## Demo and verification

```sh
rokit install
rojo build demo.project.json --output dist/demo.rbxl
rojo serve demo.project.json
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

The committed exports make the demo build independent of asset-delivery access.
Open `dist/demo.rbxl` in Studio and press Play. Walk between original and
recreated grass, lilypad and pillar. Grass is displayed at 20× on both sides to
make it easy to inspect. The multi-mesh tree was removed from `test`.
The demo runs hydration on each client because EditableMesh objects do not
replicate. Allow Mesh / Image APIs in Experience Settings for published play.

The Studio integration suite is included in the development and demo projects.
Run it from the Command Bar in Edit mode:

```luau
print(require(game.ReplicatedStorage.Tests["Runtime.spec"])())
```

Tests cover real fixed-size cloning, color alpha, cache identity, eviction,
mutable meshes, validation, sparse IDs, quads, skinning, FACS, child references,
concurrent loads and cancellation. CI runs Rust tests, strict Luau analysis,
formatting, Wally packaging and Rojo builds. Studio integration tests are manual
because GitHub runners do not provide Roblox Studio.

On Windows, `scripts/build-demo.ps1` rebuilds the executable and re-exports the
fixtures. If the native compiler cannot write its temporary files, use a writable
`TEMP`/`TMP` directory, such as `target/tmp`.

## Rokit and Wally distribution

The current source version is `0.1.0`. After publishing the matching GitHub
release and Wally package, install them with:

```toml
# rokit.toml
[tools]
rmus = "nightcycle/roblox-mesh-universal-standard@0.1.0"
```

```toml
# wally.toml
[dependencies]
MeshUniversal = "nightcycle/roblox-mesh-universal-standard@0.1.0"
```

See [.github/RELEASING.md](.github/RELEASING.md) for the release process.
Locally, `cargo install --path . --locked` installs the CLI, and Rojo can reference
`mesh-runtime/src` directly before the Wally publication exists.

## Sources

Extraction starts with the approach in
[imposter-baker](https://github.com/nightcycle/imposter-baker).
Engine semantics follow Roblox's [EditableMesh API](https://create.roblox.com/docs/reference/engine/classes/EditableMesh)
and [AssetService API](https://create.roblox.com/docs/reference/engine/classes/AssetService).
Roblox file decoding follows the
[FileMesh specification](https://devforum.roblox.com/t/roblox-filemesh-format-specification/326114).
FBX facial mappings follow [Roblox's head authoring guide](https://create.roblox.com/docs/art/characters/facial-animation/create-basic-heads).
See [NOTICE](NOTICE) for third-party attribution.
