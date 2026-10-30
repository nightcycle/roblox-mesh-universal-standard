--!strict
local Types = require(script.Types)
local Validate = require(script.Validate)
local Hydrate = require(script.Hydrate)
local Parts = require(script.Parts)
local Snapshot = require(script.Snapshot)
export type Document = Types.Document
export type Options = Types.Options
export type Source = Types.Source
export type Key = Types.Key
export type Ids = Types.Ids
export type Extension = Types.Extension

type Entry = {
	Ready: BindableEvent,
	Pending: boolean,
	Cancelled: boolean,
	FixedSize: boolean,
	Mesh: EditableMesh?,
	Document: Document?,
	Error: string?,
}
local cache: { [Key]: Entry } = {}
local extensions: { [string]: Extension } = {}
local versions: { [number]: (any) -> Document } = {}
local Mesh = {}
Mesh.Version = table.freeze({ Major = 1, Minor = 0 })

local function keyFor(source: Source, options: Options?): Key
	if typeof(source) == "Instance" then
		assert(source:IsA("ModuleScript"), "expected a ModuleScript")
		assert(not options or options.Id == nil, "ModuleScripts are keyed by instance identity; omit Id")
		return source :: ModuleScript
	end
	assert(type(source) == "table", "expected a ModuleScript or RMUS table")
	local id = if options then options.Id else nil
	assert(type(id) == "string" and #id > 0, "table sources require Options.Id")
	return id
end
local function finish(entry: Entry)
	entry.Pending = false
	entry.Ready:Fire()
	task.defer(function()
		entry.Ready:Destroy()
	end)
end
local function entryForAsync(source: Source, options: Options?): Entry
	local key = keyFor(source, options)
	local fixedSize = not options or options.FixedSize ~= false
	local existing = cache[key]
	if existing then
		assert(
			existing.FixedSize == fixedSize,
			"cache key already exists with a different FixedSize option; release it or use another key"
		)
		if existing.Pending then
			existing.Ready.Event:Wait()
		end
		assert(not existing.Cancelled, "mesh request was released while loading")
		assert(not existing.Error, existing.Error)
		assert(existing.Mesh, "mesh was released")
		return existing
	end
	local entry: Entry =
		{ Ready = Instance.new("BindableEvent"), Pending = true, Cancelled = false, FixedSize = fixedSize }
	cache[key] = entry
	local ok, message = pcall(function()
		local requireModule = require :: any
		local raw: any = if typeof(source) == "Instance" then requireModule(source) else source
		assert(type(raw) == "table" and type(raw.Version) == "table", "module must return an RMUS document")
		if raw.Version.Major ~= 1 then
			local migrate = versions[raw.Version.Major]
			assert(migrate, "unsupported RMUS major version " .. tostring(raw.Version.Major))
			raw = migrate(raw)
		end
		local document = Validate.checkAsync(raw, extensions)
		assert(not entry.Cancelled, "mesh request was released while loading")
		local editable = Hydrate.buildAsync(document, fixedSize, extensions)
		if entry.Cancelled then
			editable:Destroy()
			error("mesh request was released while loading")
		end
		entry.Mesh = editable
		entry.Document = document
	end)
	if not ok then
		entry.Error = tostring(message)
		if cache[key] == entry then
			cache[key] = nil
		end
	end
	finish(entry)
	if not ok then
		error(entry.Error :: string, 0)
	end
	return entry
end

-- May yield on first load. Cache hits share the same EditableMesh and its edits.
function Mesh.getAsync(source: Source, options: Options?): EditableMesh
	return entryForAsync(source, options).Mesh :: EditableMesh
end

-- Creates a new part each call while sharing its underlying cached mesh.
-- The caller owns the returned MeshPart.
function Mesh.createMeshPartAsync(source: Source, configuration: Configuration?, options: Options?): MeshPart
	local entry = entryForAsync(source, options)
	local part = Parts.createAsync(entry.Mesh :: EditableMesh, entry.Document :: Document, configuration)
	if entry.Cancelled then
		part:Destroy()
		error("mesh was released while creating its MeshPart", 0)
	end
	return part
end

-- Explicit eviction, not reference counting. Destroy parts using this mesh first.
function Mesh.release(key: Key): boolean
	local entry = cache[key]
	if not entry then
		return false
	end
	cache[key] = nil
	entry.Cancelled = true
	if entry.Mesh then
		entry.Mesh:Destroy()
		entry.Mesh = nil
	end
	return true
end
function Mesh.clearAsync()
	local keys = {}
	for key in cache do
		table.insert(keys, key)
	end
	for _, key in keys do
		Mesh.release(key)
	end
end
function Mesh.isCached(key: Key): boolean
	local entry = cache[key]
	return entry ~= nil and entry.Mesh ~= nil and not entry.Cancelled
end
function Mesh.validateAsync(document: any): Document
	return Validate.checkAsync(document, extensions)
end
-- Capture live data, including sparse stable IDs, into portable 1-based arrays.
function Mesh.exportAsync(editable: EditableMesh): Document
	return Snapshot.exportAsync(editable)
end
function Mesh.registerVersion(major: number, migrate: (any) -> Document)
	assert(major > 1 and major % 1 == 0 and not versions[major], "version already registered or reserved")
	versions[major] = migrate
end
function Mesh.registerExtension(name: string, hydrate: Extension)
	assert(#name > 0 and not extensions[name], "extension name is empty or already registered")
	extensions[name] = hydrate
end
return Mesh
