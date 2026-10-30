--!strict
local Types = require(script.Parent.Types)
type Document = Types.Document
local Validate = {}
local features =
	{ ["geometry"] = true, ["normals"] = true, ["colors"] = true, ["uvs"] = true, ["skinning"] = true, ["facs"] = true }

local function finite(n: any): boolean
	return type(n) == "number" and n == n and math.abs(n) < math.huge
end
local function array(value: any, label: string): { any }
	assert(type(value) == "table", label .. " must be an array")
	local count = 0
	for k in value do
		assert(type(k) == "number" and k >= 1 and k % 1 == 0, label .. " has a non-array key")
		count += 1
	end
	assert(count == #value, label .. " must be dense")
	return value
end
local function numbers(value: any, length: number, label: string)
	local a = array(value, label)
	assert(#a == length, label .. " has the wrong length")
	for _, n in a do
		assert(finite(n), label .. " contains a nonfinite number")
	end
end
local function index(n: any, count: number)
	assert(finite(n) and n % 1 == 0 and n >= 1 and n <= count, "RMUS index is outside its 1-based array")
end
local function indices(value: any, count: number, unique: boolean?)
	local seen: { [number]: boolean } = {}
	for _, n in array(value, "indices") do
		index(n, count)
		if unique then
			assert(not seen[n], "duplicate index")
		end
		seen[n] = true
	end
end
local function frame(value: any)
	numbers(value, 12, "CFrame")
	local x = Vector3.new(value[4], value[5], value[6])
	local y = Vector3.new(value[7], value[8], value[9])
	local z = Vector3.new(value[10], value[11], value[12])
	assert(
		math.abs(x:Dot(x) - 1) < 0.005
			and math.abs(y:Dot(y) - 1) < 0.005
			and math.abs(z:Dot(z) - 1) < 0.005
			and math.abs(x:Dot(y)) < 0.005
			and x:Cross(y):Dot(z) > 0.995,
		"CFrame must be rigid"
	)
end
local function unit(n: any)
	assert(finite(n) and n >= 0 and n <= 1, "color/alpha must be in 0..1")
end
function Validate.checkAsync(value: any, extensions: { [string]: Types.Extension }?): Document
	assert(type(value) == "table" and value.Format == "roblox-mesh-universal", "expected an RMUS document")
	assert(type(value.Version) == "table" and value.Version.Major == 1, "unsupported RMUS major version")
	assert(
		finite(value.Version.Minor) and value.Version.Minor >= 0 and value.Version.Minor % 1 == 0,
		"invalid RMUS minor version"
	)
	for _, f in array(value.RequiredFeatures or {}, "requiredFeatures") do
		assert(
			type(f) == "string" and (features[f] or (extensions and extensions[f])),
			"unsupported required feature: " .. tostring(f)
		)
	end
	local vertices = array(value.Vertices, "vertices")
	assert(#vertices > 0 and #vertices <= 60000, "expected 1..60000 vertices")
	for _, v in vertices do
		numbers(v, 3, "vertex")
	end
	local normals, colors, uvs =
		array(value.Normals or {}, "normals"), array(value.Colors or {}, "colors"), array(value.UVs or {}, "uvs")
	for _, n in normals do
		numbers(n, 3, "normal")
		assert(Vector3.new(n[1], n[2], n[3]).Magnitude > 0.000001, "zero normal")
	end
	for _, c in colors do
		numbers(c, 4, "color")
		for _, n in c do
			unit(n)
		end
	end
	for _, uv in uvs do
		numbers(uv, 2, "UV")
	end
	local faces = array(value.Faces, "faces")
	assert(#faces > 0, "mesh has no faces")
	local triangles = 0
	for _, f in faces do
		assert(type(f) == "table", "invalid face")
		local v = array(f.Vertices, "face vertices")
		assert(#v == 3 or #v == 4, "face must be a triangle or quad")
		indices(v, #vertices, true)
		triangles += #v - 2
		local attributeCounts: { [string]: number } = { Normals = #normals, Colors = #colors, UVs = #uvs }
		for key, count in attributeCounts do
			if f[key] and #f[key] > 0 then
				assert(#f[key] == #v, "face attribute count mismatch")
				indices(f[key], count)
			end
		end
	end
	assert(triangles <= 20000, "EditableMesh supports at most 20000 triangles")
	local bones = array(value.Bones or {}, "bones")
	local names: { [string]: boolean } = {}
	for i, b in bones do
		assert(
			type(b) == "table" and type(b.Name) == "string" and #b.Name > 0 and #b.Name <= 100 and not names[b.Name],
			"invalid/duplicate bone name"
		)
		names[b.Name] = true
		assert(b.Virtual == nil or type(b.Virtual) == "boolean", "invalid virtual flag")
		frame(b.CFrame)
		local seen: { [number]: boolean } = { [i] = true }
		local parent: any = b.Parent
		while parent do
			index(parent, #bones)
			assert(not seen[parent], "cyclic bone parent")
			seen[parent] = true
			parent = bones[parent].Parent
		end
	end
	local skinned: { [number]: boolean } = {}
	for _, s in array(value.Skin or {}, "skin") do
		index(s.Vertex, #vertices)
		assert(not skinned[s.Vertex], "duplicate skin vertex")
		skinned[s.Vertex] = true
		indices(s.Bones, #bones, true)
		assert(#s.Bones > 0 and #array(s.Weights, "weights") == #s.Bones, "skin weight count mismatch")
		local total = 0
		for _, w in s.Weights do
			unit(w)
			total += w
		end
		assert(math.abs(total - 1) < 0.001, "skin weights must sum to 1")
	end
	local poses: { [string]: boolean } = {}
	local actionNames: { [string]: boolean } = {}
	for _, e in Enum.FacsActionUnit:GetEnumItems() do
		actionNames[e.Name] = true
	end
	for _, p in array(value.Facs or {}, "facs") do
		local actions = array(p.Actions, "actions")
		assert(#actions >= 1 and #actions <= 3, "expected 1..3 FACS actions")
		local seen: { [string]: boolean } = {}
		for _, a in actions do
			assert(type(a) == "string" and actionNames[a] and not seen[a], "unknown/duplicate FACS action")
			seen[a] = true
		end
		local key = table.clone(actions)
		table.sort(key)
		local name = table.concat(key, "+")
		assert(not poses[name], "duplicate FACS pose")
		poses[name] = true
		indices(p.Bones, #bones, true)
		assert(#p.Bones == #array(p.CFrames, "FACS frames"), "FACS transform count mismatch")
		for i, id in p.Bones do
			assert(bones[id].Virtual == true, "FACS requires virtual bones")
			frame(p.CFrames[i])
		end
	end
	if value.Part then
		local p = value.Part
		numbers(p.Size, 3, "part size")
		for _, n in p.Size do
			assert(n > 0, "part size must be positive")
		end
		numbers(p.Color, 3, "part color")
		for _, n in p.Color do
			unit(n)
		end
		unit(p.Transparency)
		unit(p.Reflectance)
		frame(p.CFrame)
		frame(p.PivotOffset)
		assert(type(p.Name) == "string" and type(p.MaterialVariant) == "string", "invalid part name/material variant")
		for _, key in { "Anchored", "CanCollide", "CanTouch", "CanQuery", "CastShadow", "DoubleSided" } do
			assert(type(p[key]) == "boolean", "invalid part flag " .. key)
		end
	end
	return value :: Document
end
return Validate
