--!strict
local AssetService = game:GetService("AssetService")
local Types = require(script.Parent.Types)
local Hydrate = require(script.Parent.Hydrate)
local Parts = {}
local function enumByValue<T>(items: { T }, value: number): T
	for _, item in items do
		if (item :: any).Value == value then
			return item
		end
	end
	error("Unknown Roblox enum value " .. tostring(value))
end
local function attribute(value: any): any
	local kind, v = next(value)
	if kind == "String" or kind == "Bool" or kind == "Float32" or kind == "Float64" or kind == "Int32" then
		return v
	end
	if kind == "Vector3" then
		return Vector3.new(v[1], v[2], v[3])
	end
	if kind == "Vector2" then
		return Vector2.new(v[1], v[2])
	end
	if kind == "Color3" then
		return Color3.new(v[1], v[2], v[3])
	end
	if kind == "Color3uint8" then
		return Color3.fromRGB(v[1], v[2], v[3])
	end
	if kind == "BrickColor" then
		return BrickColor.new(v)
	end
	if kind == "UDim" then
		return UDim.new(v[1], v[2])
	end
	if kind == "UDim2" then
		return UDim2.new(v[1][1], v[1][2], v[2][1], v[2][2])
	end
	if kind == "NumberRange" then
		return NumberRange.new(v[1], v[2])
	end
	if kind == "Rect" then
		return Rect.new(v[1][1], v[1][2], v[2][1], v[2][2])
	end
	if kind == "CFrame" then
		local p, r = v.position, v.orientation
		return CFrame.new(
			p[1],
			p[2],
			p[3],
			r[1][1],
			r[1][2],
			r[1][3],
			r[2][1],
			r[2][2],
			r[2][3],
			r[3][1],
			r[3][2],
			r[3][3]
		)
	end
	if kind == "NumberSequence" then
		local points = {}
		for _, k in v.keypoints do
			table.insert(points, NumberSequenceKeypoint.new(k.time, k.value, k.envelope))
		end
		return NumberSequence.new(points)
	end
	if kind == "ColorSequence" then
		local points = {}
		for _, k in v.keypoints do
			table.insert(points, ColorSequenceKeypoint.new(k.time, Color3.new(k.color[1], k.color[2], k.color[3])))
		end
		return ColorSequence.new(points)
	end
	error("Unsupported attribute encoding " .. tostring(kind) .. "; use the exported Configuration to preserve it")
end
local function apply(part: MeshPart, p: Types.Part)
	part.Name = p.Name
	part.Size = Vector3.new(p.Size[1], p.Size[2], p.Size[3])
	part.CFrame = Hydrate.frame(p.CFrame)
	part.PivotOffset = Hydrate.frame(p.PivotOffset)
	part.Color = Color3.new(p.Color[1], p.Color[2], p.Color[3])
	part.Transparency = p.Transparency
	part.Reflectance = p.Reflectance
	part.Material = enumByValue(Enum.Material:GetEnumItems(), p.Material)
	part.MaterialVariant = p.MaterialVariant
	part.Anchored = p.Anchored
	part.CanCollide = p.CanCollide
	part.CanTouch = p.CanTouch
	part.CanQuery = p.CanQuery
	part.CastShadow = p.CastShadow
	part.DoubleSided = p.DoubleSided
end
function Parts.createAsync(mesh: EditableMesh, document: Types.Document, configuration: Configuration?): MeshPart
	local description = document.Part
	local options = {}
	if description then
		options.CollisionFidelity = enumByValue(Enum.CollisionFidelity:GetEnumItems(), description.CollisionFidelity)
		options.RenderFidelity = enumByValue(Enum.RenderFidelity:GetEnumItems(), description.RenderFidelity)
	end
	local template: MeshPart? = nil
	if configuration then
		assert(configuration:GetAttribute("__RMUSVersion") == 1, "unsupported RMUS Configuration version")
		local child = configuration:FindFirstChild("Template")
		assert(child and child:IsA("MeshPart"), "RMUS Configuration has no MeshPart Template")
		template = child
	end
	local generated = AssetService:CreateMeshPartAsync(Content.fromObject(mesh), options)
	local part: MeshPart = generated
	local ok, message = pcall(function()
		if template then
			part = (template :: MeshPart):Clone()
			local texture = part.TextureContent
			local doubleSided = part.DoubleSided
			part:ApplyMesh(generated)
			part.TextureContent = texture
			part.DoubleSided = doubleSided
			generated:Destroy()
			local byId: { [number]: Instance } = {}
			local all = part:GetDescendants()
			table.insert(all, part)
			for _, node in all do
				local id = node:GetAttribute("__RMUSNode")
				if type(id) == "number" then
					byId[id] = node
					node:SetAttribute("__RMUSNode", nil)
				end
			end
			local raw = (configuration :: Configuration):GetAttribute("__RMUSReferences")
			assert(type(raw) == "string", "invalid RMUS reference table")
			local refs = game:GetService("HttpService"):JSONDecode(raw)
			for _, ref in refs do
				local source, target = byId[ref.Source], byId[ref.Target]
				assert(source and target and type(ref.Property) == "string", "invalid RMUS reference")
				-- The property name comes from a validated Rojo model, not source code.
				local dynamicSource: any = source
				dynamicSource[ref.Property] = target
			end
		end
		if description then
			apply(part, description)
			if not template then
				for name, value in pairs(description.Attributes or {}) do
					part:SetAttribute(name, attribute(value))
				end
				for _, tag in ipairs(description.Tags or {}) do
					part:AddTag(tag)
				end
			end
		end
	end)
	if not ok then
		part:Destroy()
		if generated ~= part then
			generated:Destroy()
		end
		error(tostring(message), 0)
	end
	return part
end
return Parts
