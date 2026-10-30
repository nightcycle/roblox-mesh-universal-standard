--!strict
export type Face = { Vertices: { number }, Normals: { number }?, Colors: { number }?, UVs: { number }? }
export type Bone = { Name: string, Parent: number?, CFrame: { number }, Virtual: boolean? }
export type Skin = { Vertex: number, Bones: { number }, Weights: { number } }
export type Pose = { Actions: { string }, Bones: { number }, CFrames: { { number } } }
export type Part = {
	Name: string,
	Size: { number },
	CFrame: { number },
	Color: { number },
	Transparency: number,
	Reflectance: number,
	Material: number,
	MaterialVariant: string,
	Anchored: boolean,
	CanCollide: boolean,
	CanTouch: boolean,
	CanQuery: boolean,
	CastShadow: boolean,
	DoubleSided: boolean,
	CollisionFidelity: number,
	RenderFidelity: number,
	PivotOffset: { number },
	Attributes: { [string]: any }?,
	Tags: { string }?,
}
export type Document = {
	Format: string,
	Version: { Major: number, Minor: number },
	RequiredFeatures: { string }?,
	Vertices: { { number } },
	Faces: { Face },
	Normals: { { number } }?,
	Colors: { { number } }?,
	UVs: { { number } }?,
	Bones: { Bone }?,
	Skin: { Skin }?,
	Facs: { Pose }?,
	Part: Part?,
	Metadata: { [string]: any }?,
	Extensions: { [string]: any }?,
}
export type Options = { Id: string?, FixedSize: boolean? }
export type Source = ModuleScript | Document
export type Key = ModuleScript | string
export type Ids = {
	Vertices: { number },
	Faces: { number },
	Normals: { number },
	Colors: { number },
	UVs: { number },
	Bones: { number },
}
export type Extension = (mesh: EditableMesh, payload: any, ids: Ids) -> ()
return {}
