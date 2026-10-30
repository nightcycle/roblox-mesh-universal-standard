use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub const FORMAT: &str = "roblox-mesh-universal";
pub const FEATURES: &[&str] = &["geometry", "normals", "colors", "uvs", "skinning", "facs"];
pub type Frame = [f32; 12];
pub const IDENTITY: Frame = [0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];

// HttpService:JSONEncode emits [] for empty Luau tables, including object fields.
// Accept that unambiguous empty representation without allowing nonempty arrays.
fn object<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Value>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Object(map) => Ok(map.into_iter().collect()),
        Value::Array(values) if values.is_empty() => Ok(BTreeMap::new()),
        _ => Err(serde::de::Error::custom("expected a JSON object")),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Face {
    pub vertices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normals: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "UVs")]
    pub uvs: Vec<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Bone {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    #[serde(rename = "CFrame")]
    pub cframe: Frame,
    #[serde(default, rename = "Virtual")]
    pub is_virtual: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Skin {
    pub vertex: usize,
    pub bones: Vec<usize>,
    pub weights: Vec<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Pose {
    pub actions: Vec<String>,
    pub bones: Vec<usize>,
    #[serde(rename = "CFrames")]
    pub cframes: Vec<Frame>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Part {
    pub name: String,
    pub size: [f32; 3],
    #[serde(rename = "CFrame")]
    pub cframe: Frame,
    pub color: [f32; 3],
    pub transparency: f32,
    pub reflectance: f32,
    pub material: u32,
    pub material_variant: String,
    pub anchored: bool,
    pub can_collide: bool,
    pub can_touch: bool,
    pub can_query: bool,
    pub cast_shadow: bool,
    pub double_sided: bool,
    pub collision_fidelity: u32,
    pub render_fidelity: u32,
    pub pivot_offset: Frame,
    #[serde(default, deserialize_with = "object")]
    pub attributes: BTreeMap<String, Value>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Document {
    pub format: String,
    pub version: Version,
    #[serde(default)]
    pub required_features: Vec<String>,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Face>,
    #[serde(default)]
    pub normals: Vec<[f32; 3]>,
    #[serde(default)]
    pub colors: Vec<[f32; 4]>,
    #[serde(default)]
    #[serde(rename = "UVs")]
    pub uvs: Vec<[f32; 2]>,
    #[serde(default)]
    pub bones: Vec<Bone>,
    #[serde(default)]
    pub skin: Vec<Skin>,
    #[serde(default)]
    pub facs: Vec<Pose>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<Part>,
    #[serde(default, deserialize_with = "object")]
    pub metadata: BTreeMap<String, Value>,
    #[serde(default, deserialize_with = "object")]
    pub extensions: BTreeMap<String, Value>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            version: Version { major: 1, minor: 0 },
            required_features: vec!["geometry".into()],
            vertices: vec![],
            faces: vec![],
            normals: vec![],
            colors: vec![],
            uvs: vec![],
            bones: vec![],
            skin: vec![],
            facs: vec![],
            part: None,
            metadata: BTreeMap::new(),
            extensions: BTreeMap::new(),
        }
    }
}

fn indices(values: &[usize], count: usize) -> bool {
    values.iter().all(|&i| i > 0 && i <= count)
}
pub fn valid_frame(cf: &Frame) -> bool {
    if !cf.iter().all(|v| v.is_finite()) {
        return false;
    }
    let x = glam::Vec3::new(cf[3], cf[4], cf[5]);
    let y = glam::Vec3::new(cf[6], cf[7], cf[8]);
    let z = glam::Vec3::new(cf[9], cf[10], cf[11]);
    (x.length_squared() - 1.).abs() < 0.005
        && (y.length_squared() - 1.).abs() < 0.005
        && (z.length_squared() - 1.).abs() < 0.005
        && x.dot(y).abs() < 0.005
        && x.cross(y).dot(z) > 0.995
}
impl Document {
    pub fn require(&mut self, feature: &str) {
        if !self.required_features.iter().any(|f| f == feature) {
            self.required_features.push(feature.into());
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.format == FORMAT && self.version.major == 1,
            "unsupported format or major version"
        );
        ensure!(
            self.required_features
                .iter()
                .all(|f| FEATURES.contains(&f.as_str())),
            "unknown required feature"
        );
        ensure!(
            !self.vertices.is_empty() && self.vertices.len() <= 60000,
            "expected 1..60000 vertices"
        );
        ensure!(!self.faces.is_empty(), "mesh has no faces");
        ensure!(
            self.vertices.iter().flatten().all(|n| n.is_finite()),
            "nonfinite vertex"
        );
        ensure!(
            self.normals.iter().flatten().all(|n| n.is_finite()),
            "nonfinite normal"
        );
        ensure!(
            self.normals
                .iter()
                .all(|n| glam::Vec3::from_array(*n).length_squared() > 1e-12),
            "zero normal"
        );
        ensure!(
            self.uvs.iter().flatten().all(|n| n.is_finite()),
            "nonfinite UV"
        );
        ensure!(
            self.colors
                .iter()
                .flatten()
                .all(|n| n.is_finite() && (0.0..=1.0).contains(n)),
            "color/alpha outside 0..1"
        );
        let mut triangles = 0;
        for f in &self.faces {
            ensure!(
                (3..=4).contains(&f.vertices.len()),
                "faces must be triangles or quads"
            );
            ensure!(
                indices(&f.vertices, self.vertices.len()),
                "vertex index out of bounds (indices are 1-based)"
            );
            ensure!(
                f.vertices.iter().collect::<HashSet<_>>().len() == f.vertices.len(),
                "repeated vertex in face"
            );
            for (attr, count) in [
                (&f.normals, self.normals.len()),
                (&f.colors, self.colors.len()),
                (&f.uvs, self.uvs.len()),
            ] {
                ensure!(
                    attr.is_empty() || (attr.len() == f.vertices.len() && indices(attr, count)),
                    "invalid face corner attributes"
                );
            }
            triangles += f.vertices.len() - 2;
        }
        ensure!(
            triangles <= 20000,
            "mesh exceeds EditableMesh's 20000 triangle limit"
        );
        let mut names = HashSet::new();
        for (i, b) in self.bones.iter().enumerate() {
            ensure!(
                !b.name.is_empty() && b.name.chars().count() <= 100 && names.insert(&b.name),
                "invalid/duplicate bone name"
            );
            ensure!(
                valid_frame(&b.cframe),
                "bone CFrame must be finite and rigid"
            );
            let mut visited = HashSet::from([i + 1]);
            let mut parent = b.parent;
            while let Some(p) = parent {
                ensure!(
                    p > 0 && p <= self.bones.len() && visited.insert(p),
                    "invalid/cyclic bone parent"
                );
                parent = self.bones[p - 1].parent;
            }
        }
        let mut skinned = HashSet::new();
        for s in &self.skin {
            ensure!(
                indices(&[s.vertex], self.vertices.len()) && skinned.insert(s.vertex),
                "invalid/duplicate skin vertex"
            );
            ensure!(
                !s.bones.is_empty()
                    && s.bones.len() == s.weights.len()
                    && indices(&s.bones, self.bones.len()),
                "invalid skin indices"
            );
            ensure!(
                s.bones.iter().collect::<HashSet<_>>().len() == s.bones.len(),
                "duplicate skin bone"
            );
            ensure!(
                s.weights.iter().all(|w| w.is_finite() && *w >= 0.)
                    && (s.weights.iter().sum::<f32>() - 1.).abs() < 0.001,
                "skin weights must sum to 1"
            );
        }
        let mut poses = HashSet::new();
        for p in &self.facs {
            let database = rbx_reflection_database::get()?;
            let actions_enum = database
                .enums
                .get("FacsActionUnit")
                .ok_or_else(|| anyhow::anyhow!("reflection database has no FacsActionUnit enum"))?;
            ensure!(
                (1..=3).contains(&p.actions.len())
                    && p.actions
                        .iter()
                        .all(|a| actions_enum.items.contains_key(a.as_str())),
                "FACS pose needs 1..3 known FacsActionUnit names"
            );
            let mut actions = p.actions.clone();
            actions.sort();
            actions.dedup();
            ensure!(
                actions.len() == p.actions.len() && poses.insert(actions),
                "duplicate FACS pose/action"
            );
            ensure!(
                p.bones.len() == p.cframes.len() && indices(&p.bones, self.bones.len()),
                "invalid FACS bones"
            );
            ensure!(
                p.bones.iter().all(|i| self.bones[*i - 1].is_virtual),
                "FACS requires virtual bones"
            );
            ensure!(
                p.bones.iter().collect::<HashSet<_>>().len() == p.bones.len()
                    && p.cframes.iter().all(valid_frame),
                "invalid FACS transforms"
            );
        }
        if let Some(p) = &self.part {
            ensure!(
                p.size.iter().all(|v| v.is_finite() && *v > 0.)
                    && valid_frame(&p.cframe)
                    && valid_frame(&p.pivot_offset),
                "invalid part size/transform"
            );
            ensure!(
                p.color
                    .iter()
                    .chain([&p.transparency, &p.reflectance])
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
                "invalid part appearance"
            );
        }
        Ok(())
    }
}
