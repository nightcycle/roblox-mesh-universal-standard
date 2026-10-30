"""Generate the structural v1 JSON Schema. Semantic checks live in Rust/Luau."""
import json
from pathlib import Path

root = Path(__file__).resolve().parent.parent
number = {"type": "number"}
index = {"type": "integer", "minimum": 1}
def array(item, size=None):
    result = {"type": "array", "items": item}
    if size is not None:
        result.update(minItems=size, maxItems=size)
    return result
def obj(properties, required=()):
    return {"type": "object", "properties": properties, "required": list(required)}
def ref(name):
    return {"$ref": "#/$defs/" + name}
indices = array(index)
vector3 = array(number, 3)
frame = array(number, 12)
unit = {"type": "number", "minimum": 0, "maximum": 1}
part = {
    "Name": {"type": "string"}, "Size": array({"type": "number", "exclusiveMinimum": 0}, 3),
    "CFrame": frame, "PivotOffset": frame, "Color": array(unit, 3),
    "Transparency": unit, "Reflectance": unit, "MaterialVariant": {"type": "string"},
    **{name: {"type": "boolean"} for name in ("Anchored", "CanCollide", "CanTouch", "CanQuery", "CastShadow", "DoubleSided")},
    **{name: {"type": "integer", "minimum": 0} for name in ("Material", "CollisionFidelity", "RenderFidelity")},
}
part_required = list(part)
part.update(Attributes={"type": "object"}, Tags=array({"type": "string"}))
properties = {
    "Format": {"const": "roblox-mesh-universal"},
    "Version": obj({"Major": {"const": 1}, "Minor": {"type": "integer", "minimum": 0}}, ("Major", "Minor")),
    "RequiredFeatures": {**array({"type": "string"}), "uniqueItems": True},
    "Vertices": {**array(vector3), "minItems": 1, "maxItems": 60000},
    "Normals": array(vector3), "UVs": array(array(number, 2)), "Colors": array(array(unit, 4)),
    "Faces": {**array(ref("Face")), "minItems": 1, "maxItems": 20000},
    "Bones": array(ref("Bone")), "Skin": array(ref("Skin")), "Facs": array(ref("Pose")),
    "Part": ref("Part"), "Metadata": {"type": "object"}, "Extensions": {"type": "object"},
}
schema = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$id": "https://raw.githubusercontent.com/nightcycle/roblox-mesh-universal-standard/main/schema/mesh-v1.schema.json",
    "title": "RMUS EditableMesh interchange v1",
    **obj(properties, ("Format", "Version", "Vertices", "Faces")),
    "$defs": {
        "Face": obj({
            "Vertices": {**indices, "minItems": 3, "maxItems": 4, "uniqueItems": True},
            "Normals": indices, "Colors": indices, "UVs": indices,
        }, ("Vertices",)),
        "Bone": obj({
            "Name": {"type": "string", "minLength": 1, "maxLength": 100},
            "Parent": index, "CFrame": frame, "Virtual": {"type": "boolean"},
        }, ("Name", "CFrame")),
        "Skin": obj({
            "Vertex": index, "Bones": {**indices, "minItems": 1, "uniqueItems": True},
            "Weights": array(unit),
        }, ("Vertex", "Bones", "Weights")),
        "Pose": obj({
            "Actions": {**array({"type": "string", "minLength": 1}), "minItems": 1, "maxItems": 3, "uniqueItems": True},
            "Bones": {**indices, "uniqueItems": True}, "CFrames": array(frame),
        }, ("Actions", "Bones", "CFrames")),
        "Part": obj(part, part_required),
    },
}
(root / "schema").mkdir(exist_ok=True)
(root / "schema/mesh-v1.schema.json").write_text(json.dumps(schema, indent=2) + "\n", encoding="utf-8")
