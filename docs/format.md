# RMUS interchange v1.0

The root object has `Format: "roblox-mesh-universal"` and
`Version: { "Major": 1, "Minor": 0 }`. Major changes are incompatible.
Minor changes add optional data without changing existing meanings.
Consumers reject unknown major versions and unknown `RequiredFeatures`.
The Rust reader also accepts `[]` for empty object fields because Roblox's
`HttpService:JSONEncode` emits that representation for empty Luau tables.
Canonical exports always write JSON objects for metadata and extensions.
All RMUS-defined data fields use PascalCase, matching the Wally package types.
Developer-defined metadata, extension payloads, and attribute names keep their
original keys. Roblox attribute encodings and Rojo's model schema keep the
field names required by those external formats.

All numbers must be finite. Position units are Roblox studs. Axes are right-handed,
Y up, -Z forward. Colors are RGB values passed directly to `Color3.new` plus alpha
(1 opaque, 0 transparent), all in [0,1]. The exporter does not convert vertex
colors between transfer functions or multiply them by MeshPart.Color.

All cross-array references start at **1**. Never store Roblox's stable element IDs
as indices; they may be sparse, tagged and much larger than the array length.

| Field | Shape / semantics |
| --- | --- |
| `Vertices` | Array of [x,y,z]. 1–60,000 vertices. |
| `Normals` | Optional array of nonzero [x,y,z]. |
| `Colors` | Optional array of [r,g,b,alpha]. |
| `UVs` | Optional array of [u,v], Roblox UV convention. No images. |
| `Faces` | Array of objects with `Vertices` (3 or 4 indices), optional `Normals`, `Colors`, `UVs` arrays of matching corner count. 20,000 triangle equivalents maximum; a quad counts as two. |
| `Bones` | Array of `{Name,Parent?,CFrame,Virtual?}`. Names are unique. Parent references the bones array; omitted means root. Bind CFrames are in mesh space, not parent space. |
| `Skin` | Array of `{Vertex,Bones,Weights}`. One entry per weighted vertex; unique bone indices; normalized nonnegative weights. |
| `Facs` | Array of `{Actions,Bones,CFrames}`. One action is a base pose; two or three are a corrective. Actions are FacsActionUnit names. Bones must be virtual. CFrames are complete mesh-space transformations from bind to target, as consumed by EditableMesh. |
| `Part` | Optional common MeshPart properties: size, CFrame, pivot, color, alpha via transparency, material, flags and fidelity. Attributes use rbx_types tagged JSON; tags are strings. Extra properties and descendants live in the optional Configuration. |
| `Metadata` | Developer-defined JSON object. Not interpreted during hydration. |
| `Extensions` | Namespaced payload objects for future capabilities. |
| `RequiredFeatures` | Feature identifiers that must be understood: geometry, normals, colors, uvs, skinning, facs, or registered extension names. |

CFrames contain twelve numbers in the order accepted by `CFrame.new`:
`[x,y,z,r00,r01,r02,r10,r11,r12,r20,r21,r22]`.
Rotation matrices must be right-handed and orthonormal.

Normals, colors and UVs reference independent arrays per face corner. Vertices are
not split just to represent a sharp edge, color seam or UV seam. Empty or omitted
face attribute arrays use Roblox's automatically generated defaults.

The exporter normalizes byte-quantized skin weights. Roblox may quantize them
again during hydration; a subsequent live export normalizes the returned weights.
Geometry values may also differ by normal floating-point/Draco quantization.

Roblox v5+ files encode FACS corrective *deltas*. Export resolves those together
with their active base/subset corrections into complete corrective targets.
FBX `FrameN` properties already identify full posed frames. Unknown chunked
Roblox data is retained as `Extensions["roblox.chunk.<NAME>"]` with `Version`
and `Base64`. These optional source chunks have no current EditableMesh setter.
Lower render LODs are not overlaid on the highest-detail topology.

Extension example:

```lua
Mesh.registerExtension("my-studio.future-attribute", function(editable, payload, ids)
    -- ids.Vertices[i], ids.Faces[i], ids.Bones[i], etc. are actual engine IDs.
    -- Populate a supported future engine capability before FixedSize cloning.
end)
```

Optional unknown fields can be ignored. A producer that requires a feature to
render correctly must list it in `RequiredFeatures`. A registered future-major
migration must return a valid v1 document. This separates interchange versioning
from the Rust crate and Wally package release versions.
