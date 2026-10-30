# Roblox Mesh Universal Standard runtime

`nightcycle/roblox-mesh-universal-standard` provides strictly typed EditableMesh
hydration for RMUS v1.
Interface functions use camelCase with `.`, and instance methods use PascalCase
with `:`. Potentially yielding or expensive functions end in `Async` and return
their results directly. All defined data fields use PascalCase.
Full format and exporter documentation:
https://github.com/nightcycle/roblox-mesh-universal-standard

```lua
local Mesh = require(game.ReplicatedStorage.Packages.MeshUniversal)
local editable = Mesh.getAsync(dataModule)
local part = Mesh.createMeshPartAsync(dataModule, childrenConfiguration)
part.Parent = workspace
part:Destroy()
Mesh.release(dataModule)

local mutable = Mesh.getAsync(decodedTable, { Id = "my-mesh", FixedSize = false })
Mesh.release("my-mesh")
```

FixedSize defaults to true. ModuleScript identity or an explicit table ID selects
the cache entry. Repeated getAsync calls do not increment a reference count.
release destroys the shared EditableMesh; destroy its MeshParts first.
createMeshPartAsync creates a separate caller-owned part each call.
Enable Allow Mesh / Image APIs for published experiences.

Also available: validateAsync, exportAsync, isCached, clearAsync, registerVersion,
registerExtension. All exported Luau types are available through the package.
Future required capabilities fail explicitly instead of silently dropping data.
