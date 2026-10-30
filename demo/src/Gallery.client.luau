--!strict
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Mesh = require(ReplicatedStorage.MeshUniversal)
local exports = ReplicatedStorage.Exports
local originals = ReplicatedStorage.Originals
local gallery = Instance.new("Folder")
gallery.Name = "MeshComparisonGallery"
gallery.Parent = workspace

local gui = Instance.new("ScreenGui")
gui.Name = "MeshGallery"
gui.ResetOnSpawn = false
local panel = Instance.new("TextLabel")
panel.Size = UDim2.new(0, 560, 0, 116)
panel.Position = UDim2.new(0, 24, 0, 24)
panel.BackgroundColor3 = Color3.fromRGB(21, 29, 35)
panel.BackgroundTransparency = 0.08
panel.TextColor3 = Color3.fromRGB(231, 240, 245)
panel.Font = Enum.Font.Gotham
panel.TextSize = 17
panel.TextWrapped = true
panel.Text =
	"MESH UNIVERSAL • COMPARISON GALLERY\nOriginals on the left · Recreated meshes on the right\nWASD to walk · Space to jump\nLoading exported mesh data…"
panel.Parent = gui
local corner = Instance.new("UICorner")
corner.CornerRadius = UDim.new(0, 12)
corner.Parent = panel
gui.Parent = Players.LocalPlayer:WaitForChild("PlayerGui")

local function label(text: string, position: Vector3, color: Color3)
	local stand = Instance.new("Part")
	stand.Name = text
	stand.Anchored = true
	stand.CanCollide = false
	stand.Size = Vector3.new(18, 0.25, 14)
	stand.Position = position
	stand.Color = color
	stand.Material = Enum.Material.SmoothPlastic
	stand.Parent = gallery
	local billboard = Instance.new("BillboardGui")
	billboard.Size = UDim2.fromOffset(300, 60)
	billboard.StudsOffsetWorldSpace = Vector3.new(0, 5, 0)
	billboard.MaxDistance = 100
	billboard.Adornee = stand
	billboard.AlwaysOnTop = true
	billboard.Parent = stand
	local textLabel = Instance.new("TextLabel")
	textLabel.Size = UDim2.fromScale(1, 1)
	textLabel.BackgroundTransparency = 1
	textLabel.Text = text
	textLabel.TextColor3 = Color3.new(1, 1, 1)
	textLabel.TextStrokeTransparency = 0.4
	textLabel.Font = Enum.Font.GothamMedium
	textLabel.TextSize = 17
	textLabel.Parent = billboard
	return stand
end
local function meshPart(root: Instance): MeshPart
	if root:IsA("MeshPart") then
		return root
	end
	local found = root:FindFirstChildWhichIsA("MeshPart", true)
	assert(found and found:IsA("MeshPart"), "original has no MeshPart")
	return found
end
local function position(part: MeshPart, x: number, z: number, scale: number)
	part.Anchored = true
	part.Size *= scale
	-- Preserve orientation and compute its vertical extent.
	local rotation = part.CFrame.Rotation
	local size = part.Size
	local halfHeight = (
		math.abs(rotation.XVector.Y) * size.X
		+ math.abs(rotation.YVector.Y) * size.Y
		+ math.abs(rotation.ZVector.Y) * size.Z
	) / 2
	-- Keep the bottom of each mesh about one stud above its display platform.
	part.CFrame = CFrame.new(x, 1.3 + halfHeight, z) * rotation
end
local successes = 0
local errors = {}
for i, name in { "Grass", "Lilypad", "Pillar" } do
	local z = 12 - (i - 1) * 28
	local scale = if name == "Grass" then 20 else 1
	local title = name .. if scale > 1 then " ×20" else ""
	label(title .. " · ORIGINAL", Vector3.new(-14, 0.15, z), Color3.fromRGB(62, 84, 100))
	label(title .. " · RECREATED", Vector3.new(14, 0.15, z), Color3.fromRGB(35, 106, 91))
	local original = originals:FindFirstChild(name)
	assert(original, name .. " original missing")
	local clone = original:Clone()
	position(meshPart(clone), -14, z, scale)
	clone.Parent = gallery
	local module = exports:FindFirstChild(name)
	local children = exports:FindFirstChild(name .. "Children")
	assert(module and module:IsA("ModuleScript"))
	assert(children and children:IsA("Configuration"))
	local ok, message = pcall(function()
		local editable = Mesh.getAsync(module)
		assert(editable.FixedSize and editable == Mesh.getAsync(module), "cache/fixed-size check failed")
		local recreated = Mesh.createMeshPartAsync(module, children)
		position(recreated, 14, z, scale)
		recreated.Parent = gallery
		successes += 1
	end)
	if not ok then
		table.insert(errors, name .. ": " .. tostring(message))
		warn("Mesh gallery:", name, message)
	end
	task.wait()
end
panel.Text = "MESH UNIVERSAL • COMPARISON GALLERY\nOriginals on the left · Recreated meshes on the right\nWASD to walk · Space to jump\n"
	.. if successes == 3
		then "3 / 3 hydrated · FixedSize enabled · Shared cache verified"
		else tostring(successes)
			.. " / 3 hydrated. Enable Allow Mesh / Image APIs in Experience Settings. See Output for details."
script.Destroying:Connect(function()
	gallery:Destroy()
	gui:Destroy()
	Mesh.clearAsync()
end)
