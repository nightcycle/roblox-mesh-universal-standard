--!strict
local AssetService = game:GetService("AssetService")
local Types = require(script.Parent.Types)
local Hydrate = {}
function Hydrate.frame(v: { number }): CFrame
	return CFrame.new(v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], v[10], v[11], v[12])
end
local function map(indices: { number }, ids: { number }): { number }
	local result = table.create(#indices)
	for i, id in indices do
		result[i] = ids[id]
	end
	return result
end
function Hydrate.buildAsync(
	document: Types.Document,
	fixedSize: boolean,
	extensions: { [string]: Types.Extension }
): EditableMesh
	local mesh = AssetService:CreateEditableMesh()
	assert(mesh, "Roblox could not allocate an EditableMesh; its memory budget may be exhausted")
	local result: EditableMesh? = nil
	local ok, message = pcall(function()
		local ids: Types.Ids = { Vertices = {}, Faces = {}, Normals = {}, Colors = {}, UVs = {}, Bones = {} }
		for i, v in document.Vertices do
			ids.Vertices[i] = mesh:AddVertex(Vector3.new(v[1], v[2], v[3]))
		end
		for i, n in ipairs(document.Normals or {}) do
			ids.Normals[i] = mesh:AddNormal(Vector3.new(n[1], n[2], n[3]))
		end
		for i, c in ipairs(document.Colors or {}) do
			ids.Colors[i] = mesh:AddColor(Color3.new(c[1], c[2], c[3]), c[4])
		end
		for i, uv in ipairs(document.UVs or {}) do
			ids.UVs[i] = mesh:AddUV(Vector2.new(uv[1], uv[2]))
		end
		for i, f in document.Faces do
			local v = map(f.Vertices, ids.Vertices)
			local id = if #v == 3 then mesh:AddTriangle(v[1], v[2], v[3]) else mesh:AddFace(table.unpack(v))
			ids.Faces[i] = id
			if f.Normals and #f.Normals > 0 then
				mesh:SetFaceNormals(id, map(f.Normals, ids.Normals))
			end
			if f.Colors and #f.Colors > 0 then
				mesh:SetFaceColors(id, map(f.Colors, ids.Colors))
			end
			if f.UVs and #f.UVs > 0 then
				mesh:SetFaceUVs(id, map(f.UVs, ids.UVs))
			end
		end
		local bones: { Types.Bone } = document.Bones or {}
		local function addBone(i: number): number
			if ids.Bones[i] then
				return ids.Bones[i]
			end
			local b = bones[i]
			local parent = if b.Parent then addBone(b.Parent) else nil
			local id = mesh:AddBone({
				Name = b.Name,
				CFrame = Hydrate.frame(b.CFrame),
				ParentId = parent,
				Virtual = b.Virtual == true,
			})
			ids.Bones[i] = id
			return id
		end
		for i in bones do
			addBone(i)
		end
		for _, s in ipairs(document.Skin or {}) do
			mesh:SetVertexBones(ids.Vertices[s.Vertex], map(s.Bones, ids.Bones))
			mesh:SetVertexBoneWeights(ids.Vertices[s.Vertex], s.Weights)
		end
		local actionByName: { [string]: Enum.FacsActionUnit } = {}
		for _, action in Enum.FacsActionUnit:GetEnumItems() do
			actionByName[action.Name] = action
		end
		for _, pose in ipairs(document.Facs or {}) do
			local actions, frames = {}, {}
			for i, name in pose.Actions do
				actions[i] = actionByName[name]
			end
			for i, cf in pose.CFrames do
				frames[i] = Hydrate.frame(cf)
			end
			if #actions == 1 then
				mesh:SetFacsPose(actions[1], map(pose.Bones, ids.Bones), frames)
			else
				mesh:SetFacsCorrectivePose(actions, map(pose.Bones, ids.Bones), frames)
			end
		end
		for name, payload in pairs(document.Extensions or {}) do
			local handler = extensions[name]
			if handler then
				handler(mesh, payload, ids)
			end
		end
		if fixedSize then
			result = AssetService:CreateEditableMeshAsync(Content.fromObject(mesh), { FixedSize = true })
			assert(result, "Roblox could not allocate the fixed-size EditableMesh")
			assert((result :: EditableMesh).FixedSize, "Roblox returned a non-fixed mesh")
		else
			result = mesh
		end
	end)
	if not ok then
		if result and result ~= mesh then
			result:Destroy()
		end
		mesh:Destroy()
		error(tostring(message), 0)
	end
	if fixedSize then
		mesh:Destroy()
	end
	return result :: EditableMesh
end
return Hydrate
