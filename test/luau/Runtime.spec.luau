--!strict
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local AssetService = game:GetService("AssetService")
local Mesh = require(ReplicatedStorage.MeshUniversal)
local testRun = 0
local function triangle(): Mesh.Document
	return {
		Format = "roblox-mesh-universal",
		Version = { Major = 1, Minor = 0 },
		Vertices = { { 0, 0, 0 }, { 1, 0, 0 }, { 0, 1, 0 } },
		Faces = { { Vertices = { 1, 2, 3 }, Normals = { 1, 1, 1 }, Colors = { 1, 2, 1 } } },
		Normals = { { 0, 0, 1 } },
		Colors = { { 1, 0, 0, 0.25 }, { 0, 1, 0, 1 } },
	}
end
local function rejects(callback: () -> ())
	assert(not pcall(callback), "expected rejection")
end
return function(): { string }
	testRun += 1
	Mesh.clearAsync()
	local passed = {}
	local function test(name: string, callback: () -> ())
		local ok, message = pcall(function()
			callback()
			return true
		end)
		Mesh.clearAsync()
		assert(ok, name .. ": " .. tostring(message))
		table.insert(passed, name)
	end
	test("fixed-size, color alpha, cache identity and eviction", function()
		local data = triangle()
		local a = Mesh.getAsync(data, { Id = "one" })
		assert(a.FixedSize and a == Mesh.getAsync(triangle(), { Id = "one" }))
		local exported = Mesh.exportAsync(a)
		local face = exported.Faces[1]
		local c = (exported.Colors :: { { number } })[(face.Colors :: { number })[1]]
		assert(math.abs(c[4] - 0.25) < 0.005 and c[1] == 1)
		rejects(function()
			Mesh.getAsync(data, { Id = "one", FixedSize = false })
		end)
		assert(Mesh.release("one") and not Mesh.release("one") and not Mesh.isCached("one"))
		assert(Mesh.getAsync(data, { Id = "one" }) ~= a)
	end)
	test("mutable meshes and required IDs", function()
		local data = triangle()
		rejects(function()
			Mesh.getAsync(data)
		end)
		local a = Mesh.getAsync(data, { Id = "mutable", FixedSize = false })
		assert(not a.FixedSize)
		a:AddVertex(Vector3.new(0, 0, 1))
		assert(#a:GetVertices() == 4)
	end)
	test("validation prevents invalid topology, versions and skinning", function()
		local data = triangle()
		data.Faces[1].Vertices[1] = 0
		rejects(function()
			Mesh.getAsync(data, { Id = "bad" })
		end)
		data = triangle()
		data.Version.Major = 999
		rejects(function()
			Mesh.getAsync(data, { Id = "bad" })
		end)
		data = triangle()
		data.RequiredFeatures = { "unknown-required-feature" }
		rejects(function()
			Mesh.getAsync(data, { Id = "bad" })
		end)
		assert(not Mesh.isCached("bad"))
	end)
	test("quad, sparse IDs, skinning and FACS round trip", function()
		local original = AssetService:CreateEditableMesh()
		assert(original)
		local discarded = original:AddVertex(Vector3.new(50, 50, 50))
		original:RemoveUnused()
		local v = {}
		for _, p in { Vector3.zero, Vector3.xAxis, Vector3.new(1, 1, 0), Vector3.yAxis } do
			table.insert(v, original:AddVertex(p))
		end
		assert(v[1] ~= discarded)
		original:AddFace(table.unpack(v))
		local root = original:AddBone({ Name = "Root", CFrame = CFrame.identity, Virtual = false })
		local faceBone =
			original:AddBone({ Name = "Face", ParentId = root, CFrame = CFrame.new(0, 1, 0), Virtual = true })
		for _, id in v do
			original:SetVertexBones(id, { root, faceBone })
			original:SetVertexBoneWeights(id, { 0.25, 0.75 })
		end
		original:SetFacsPose(Enum.FacsActionUnit.JawDrop, { faceBone }, { CFrame.new(0, -0.25, 0) })
		original:SetFacsPose(Enum.FacsActionUnit.Funneler, { faceBone }, { CFrame.new(0, 0, 0.1) })
		original:SetFacsPose(Enum.FacsActionUnit.Pucker, { faceBone }, { CFrame.new(0, 0, 0.2) })
		original:SetFacsCorrectivePose(
			{ Enum.FacsActionUnit.JawDrop, Enum.FacsActionUnit.Funneler },
			{ faceBone },
			{ CFrame.new(0, -0.3, 0.2) }
		)
		original:SetFacsCorrectivePose(
			{ Enum.FacsActionUnit.JawDrop, Enum.FacsActionUnit.Funneler, Enum.FacsActionUnit.Pucker },
			{ faceBone },
			{ CFrame.new(0, -0.4, 0.3) }
		)
		local data = Mesh.exportAsync(original)
		local rebuilt = Mesh.getAsync(data, { Id = "roundtrip" })
		assert(#rebuilt:GetVertices() == 4 and #rebuilt:GetFaces() == 1)
		assert(#rebuilt:GetBones() == 2 and #rebuilt:GetFacsPoses() == 3 and #rebuilt:GetFacsCorrectivePoses() == 2)
		local copied = Mesh.exportAsync(rebuilt)
		assert(#(copied.Skin :: { any }) == 4 and #(copied.Facs :: { any }) == 5)
		assert(math.abs((copied.Skin :: { any })[1].Weights[2] - 0.75) < 0.005)
		original:Destroy()
	end)
	test("concurrent cache requests share one hydration", function()
		local data = triangle()
		local a: EditableMesh? = nil
		local b: EditableMesh? = nil
		local failures = {}
		local remaining = 2
		for i = 1, 2 do
			task.spawn(function()
				local ok, message = pcall(function()
					local mesh = Mesh.getAsync(data, { Id = "race" })
					if i == 1 then
						a = mesh
					else
						b = mesh
					end
				end)
				if not ok then
					table.insert(failures, tostring(message))
				end
				remaining -= 1
			end)
		end
		while remaining > 0 do
			task.wait()
		end
		assert(#failures == 0, table.concat(failures, "\n"))
		assert(a ~= nil and a == b)
	end)
	test("configuration preserves children, attributes and internal references", function()
		local config = ReplicatedStorage:FindFirstChild("TestConfiguration")
		assert(config and config:IsA("Configuration"))
		local part = Mesh.createMeshPartAsync(triangle(), config, { Id = "children" })
		assert(part:GetAttribute("Example") == "preserved")
		local tip = part:FindFirstChild("Tip")
		assert(tip and tip:IsA("Attachment") and tip.Position == Vector3.new(0, 2, 0))
		local tipRef = part:FindFirstChild("TipReference")
		local selfRef = part:FindFirstChild("SelfReference")
		assert(tipRef and tipRef:IsA("ObjectValue") and tipRef.Value == tip)
		assert(selfRef and selfRef:IsA("ObjectValue") and selfRef.Value == part)
		assert(part:FindFirstChildWhichIsA("SurfaceAppearance"))
		local root = part:FindFirstChild("Root")
		assert(root and root:IsA("Bone") and root:FindFirstChild("Child"))
		assert(part:GetAttribute("__RMUSNode") == nil)
		part:Destroy()
		assert(config:FindFirstChild("Template"), "source configuration must be reusable")
	end)
	test("release during yielding hydration cancels and permits retry", function()
		local extension = "test.yield." .. tostring(testRun)
		Mesh.registerExtension(extension, function()
			task.wait(0.1)
		end)
		local data = triangle()
		data.Extensions = { [extension] = true }
		local finished = false
		local succeeded = true
		task.spawn(function()
			succeeded = pcall(function()
				Mesh.getAsync(data, { Id = "cancel" })
			end)
			finished = true
		end)
		task.wait()
		assert(Mesh.release("cancel"))
		while not finished do
			task.wait()
		end
		assert(not succeeded and not Mesh.isCached("cancel"))
		data.Extensions = {}
		assert(Mesh.getAsync(data, { Id = "cancel" }))
	end)
	return passed
end
