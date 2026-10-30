--!strict
local Types = require(script.Parent.Types)
local Snapshot = {}
local function map(ids: { number }): { [number]: number }
	local lookup = {}
	for i, id in ids do
		lookup[id] = i
	end
	return lookup
end
local function indices(ids: { number }, lookup: { [number]: number }): { number }
	local result = table.create(#ids)
	for i, id in ids do
		local index = lookup[id]
		assert(index, "EditableMesh returned an unknown element ID")
		result[i] = index
	end
	return result
end
local function frames(cframes: { CFrame }): { { number } }
	local result = {}
	for i, cf in cframes do
		result[i] = { cf:GetComponents() }
	end
	return result
end
function Snapshot.exportAsync(mesh: EditableMesh): Types.Document
	local vertexIds, faceIds = mesh:GetVertices(), mesh:GetFaces()
	local normalIds, colorIds, uvIds, boneIds = mesh:GetNormals(), mesh:GetColors(), mesh:GetUVs(), mesh:GetBones()
	for _, ids in { vertexIds, faceIds, normalIds, colorIds, uvIds, boneIds } do
		table.sort(ids)
	end
	local vertexMap, normalMap, colorMap, uvMap, boneMap =
		map(vertexIds), map(normalIds), map(colorIds), map(uvIds), map(boneIds)
	local vertices, normals, colors, uvs = {}, {}, {}, {}
	local faces: { Types.Face } = {}
	local bones: { Types.Bone } = {}
	local skin: { Types.Skin } = {}
	local facs: { Types.Pose } = {}
	for i, id in vertexIds do
		local v = mesh:GetPosition(id)
		vertices[i] = { v.X, v.Y, v.Z }
		local attached = mesh:GetVertexBones(id)
		if #attached > 0 then
			-- Roblox quantizes each weight to a byte independently; their returned
			-- sum can differ from one. Normalize for portable interchange.
			local weights = mesh:GetVertexBoneWeights(id)
			local total = 0
			for _, weight in weights do
				total += weight
			end
			assert(total > 0, "skinned vertex has zero total weight")
			for j, weight in weights do
				weights[j] = weight / total
			end
			table.insert(skin, { Vertex = i, Bones = indices(attached, boneMap), Weights = weights })
		end
	end
	for i, id in normalIds do
		local n = mesh:GetNormal(id)
		assert(n)
		normals[i] = { n.X, n.Y, n.Z }
	end
	for i, id in colorIds do
		local c, a = mesh:GetColor(id), mesh:GetColorAlpha(id)
		assert(c and a)
		colors[i] = { c.R, c.G, c.B, a }
	end
	for i, id in uvIds do
		local uv = mesh:GetUV(id)
		assert(uv)
		uvs[i] = { uv.X, uv.Y }
	end
	for i, id in faceIds do
		faces[i] = {
			Vertices = indices(mesh:GetFaceVertices(id), vertexMap),
			Normals = indices(mesh:GetFaceNormals(id), normalMap),
			Colors = indices(mesh:GetFaceColors(id), colorMap),
			UVs = indices(mesh:GetFaceUVs(id), uvMap),
		}
	end
	for i, id in boneIds do
		bones[i] = {
			Name = mesh:GetBoneName(id),
			Parent = boneMap[mesh:GetBoneParent(id)],
			CFrame = { mesh:GetBoneCFrame(id):GetComponents() },
			Virtual = mesh:GetBoneIsVirtual(id),
		}
	end
	for _, action in mesh:GetFacsPoses() do
		local ids, cframes = mesh:GetFacsPose(action)
		table.insert(facs, { Actions = { action.Name }, Bones = indices(ids, boneMap), CFrames = frames(cframes) })
	end
	for _, actions in mesh:GetFacsCorrectivePoses() do
		local names = {}
		for i, action in actions do
			names[i] = action.Name
		end
		local ids, cframes = mesh:GetFacsCorrectivePose(actions)
		table.insert(facs, { Actions = names, Bones = indices(ids, boneMap), CFrames = frames(cframes) })
	end
	return {
		Format = "roblox-mesh-universal",
		Version = { Major = 1, Minor = 0 },
		RequiredFeatures = { "geometry", "normals", "colors", "uvs", "skinning", "facs" },
		Vertices = vertices,
		Faces = faces,
		Normals = normals,
		Colors = colors,
		UVs = uvs,
		Bones = bones,
		Skin = skin,
		Facs = facs,
	}
end
return Snapshot
