import json
from pathlib import Path
import jsonschema

root = Path(__file__).resolve().parent.parent
schema = json.loads((root / "schema/mesh-v1.schema.json").read_text())
jsonschema.Draft202012Validator.check_schema(schema)
files = [path for path in (root / "demo/assets").glob("*.json") if not path.name.endswith(".model.json")]
assert len(files) == 3
for path in files:
    jsonschema.validate(json.loads(path.read_text()), schema)
print("JSON Schema and all three demo exports passed")
