use crate::schema::{Frame, Part};
use anyhow::{Context, Result, bail, ensure};
use rbx_dom_weak::{
    Instance, WeakDom,
    types::{CFrame, Ref, Variant},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    io::Cursor,
};

pub struct Model {
    pub dom: WeakDom,
}
pub struct SourceMesh {
    pub content: String,
    pub part: Part,
}

pub fn frame(v: CFrame) -> Frame {
    let r = v.orientation;
    [
        v.position.x,
        v.position.y,
        v.position.z,
        r.x.x,
        r.x.y,
        r.x.z,
        r.y.x,
        r.y.y,
        r.y.z,
        r.z.x,
        r.z.y,
        r.z.z,
    ]
}
fn prop<'a>(i: &'a Instance, name: &str) -> Option<&'a Variant> {
    i.properties.get(&name.into()).or_else(|| {
        let db = rbx_reflection_database::get().ok()?;
        db.classes
            .get(i.class.as_str())
            .and_then(|c| db.find_default_property(c, name))
    })
}
fn vector(i: &Instance, key: &str, fallback: [f32; 3]) -> [f32; 3] {
    match prop(i, key) {
        Some(Variant::Vector3(v)) => [v.x, v.y, v.z],
        _ => fallback,
    }
}
fn cf(i: &Instance, key: &str) -> Frame {
    match prop(i, key) {
        Some(Variant::CFrame(v)) => frame(*v),
        _ => crate::schema::IDENTITY,
    }
}
fn number(i: &Instance, key: &str, default: f32) -> f32 {
    match prop(i, key) {
        Some(Variant::Float32(v)) => *v,
        Some(Variant::Float64(v)) => *v as f32,
        _ => default,
    }
}
fn boolean(i: &Instance, key: &str, default: bool) -> bool {
    match prop(i, key) {
        Some(Variant::Bool(v)) => *v,
        _ => default,
    }
}
fn en(i: &Instance, key: &str, default: u32) -> u32 {
    match prop(i, key) {
        Some(Variant::Enum(v)) => v.to_u32(),
        _ => default,
    }
}
fn string(i: &Instance, key: &str) -> String {
    match prop(i, key) {
        Some(Variant::String(v)) => v.clone(),
        _ => String::new(),
    }
}
fn color(i: &Instance) -> [f32; 3] {
    match prop(i, "Color") {
        Some(Variant::Color3(v)) => [v.r, v.g, v.b],
        Some(Variant::Color3uint8(v)) => [v.r as f32 / 255., v.g as f32 / 255., v.b as f32 / 255.],
        _ => [163. / 255.; 3],
    }
}
pub fn content(i: &Instance, names: &[&str]) -> Option<String> {
    names.iter().find_map(|key| match prop(i, key)? {
        Variant::Content(v) => v.as_uri().filter(|s| !s.is_empty()).map(str::to_owned),
        Variant::ContentId(v) => Some(v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        Variant::String(v) => Some(v).filter(|s| !s.is_empty()).cloned(),
        _ => None,
    })
}
impl Model {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let dom = if bytes.starts_with(b"<roblox!") {
            rbx_binary::from_reader(Cursor::new(bytes))?
        } else {
            rbx_xml::from_reader(Cursor::new(bytes), rbx_xml::DecodeOptions::default())?
        };
        Ok(Self { dom })
    }
    pub fn mesh_instances(&self) -> Vec<&Instance> {
        self.dom
            .descendants()
            .filter(|i| matches!(i.class.as_str(), "MeshPart" | "SpecialMesh"))
            .collect()
    }
    fn single(&self) -> Result<&Instance> {
        let meshes = self.mesh_instances();
        ensure!(
            meshes.len() == 1,
            "expected exactly one mesh instance; found {}",
            meshes.len()
        );
        Ok(meshes[0])
    }
    pub fn mesh(&self) -> Result<SourceMesh> {
        let i = self.single()?;
        let owner = if i.class == "SpecialMesh" {
            ensure!(en(i, "MeshType", 5) == 5, "SpecialMesh must be FileMesh");
            self.dom
                .get_by_ref(i.parent())
                .context("SpecialMesh has no parent")?
        } else {
            i
        };
        let mut attributes = BTreeMap::new();
        if let Some(Variant::Attributes(a)) = prop(owner, "Attributes") {
            for (key, value) in a.iter() {
                attributes.insert(key.clone(), serde_json::to_value(value)?);
            }
        }
        let tags = match prop(owner, "Tags") {
            Some(Variant::Tags(v)) => v.iter().map(str::to_owned).collect(),
            _ => vec![],
        };
        let mut part = Part {
            name: owner.name.clone(),
            size: vector(owner, "Size", [1.; 3]),
            cframe: cf(owner, "CFrame"),
            color: color(owner),
            transparency: number(owner, "Transparency", 0.),
            reflectance: number(owner, "Reflectance", 0.),
            material: en(owner, "Material", 256),
            material_variant: string(owner, "MaterialVariant"),
            anchored: boolean(owner, "Anchored", false),
            can_collide: boolean(owner, "CanCollide", true),
            can_touch: boolean(owner, "CanTouch", true),
            can_query: boolean(owner, "CanQuery", true),
            cast_shadow: boolean(owner, "CastShadow", true),
            double_sided: boolean(owner, "DoubleSided", false),
            collision_fidelity: en(owner, "CollisionFidelity", 0),
            render_fidelity: en(owner, "RenderFidelity", 0),
            pivot_offset: cf(owner, "PivotOffset"),
            attributes,
            tags,
        };
        if i.class == "SpecialMesh" {
            // SpecialMesh geometry uses Scale and Offset, not the parent's Size.
            // Handled by transform_special after decoding.
            part.color = vector(i, "VertexColor", part.color);
        }
        Ok(SourceMesh {
            content: content(i, &["MeshContent", "MeshId"])
                .context("mesh has no downloadable MeshId")?,
            part,
        })
    }
    pub fn transform_special(&self, doc: &mut crate::schema::Document) -> Result<()> {
        let i = self.single()?;
        if i.class != "SpecialMesh" {
            return Ok(());
        }
        let scale = glam::Vec3::from_array(vector(i, "Scale", [1.; 3]));
        let offset = glam::Vec3::from_array(vector(i, "Offset", [0.; 3]));
        ensure!(
            scale.min_element() > 0.,
            "SpecialMesh Scale must be positive"
        );
        ensure!(
            doc.bones.is_empty(),
            "skinned SpecialMesh is not supported by Roblox"
        );
        for p in &mut doc.vertices {
            *p = (glam::Vec3::from_array(*p) * scale + offset).to_array();
        }
        for n in &mut doc.normals {
            *n = (glam::Vec3::from_array(*n) / scale).normalize().to_array();
        }
        let lo = doc
            .vertices
            .iter()
            .fold(glam::Vec3::splat(f32::INFINITY), |a, b| a.min((*b).into()));
        let hi = doc
            .vertices
            .iter()
            .fold(glam::Vec3::splat(f32::NEG_INFINITY), |a, b| {
                a.max((*b).into())
            });
        if let Some(p) = &mut doc.part {
            p.size = (hi - lo).max(glam::Vec3::splat(0.001)).to_array();
        }
        Ok(())
    }
    /// Configuration contains a property template and its descendants. References
    /// are restored by node IDs because Rojo JSON models cannot encode arbitrary
    /// references among children.
    pub fn sidecar(&self) -> Result<Value> {
        let mesh = self.single()?;
        let owner = if mesh.class == "SpecialMesh" {
            self.dom.get_by_ref(mesh.parent()).unwrap()
        } else {
            mesh
        };
        let mut ids = HashMap::new();
        fn visit(dom: &WeakDom, i: &Instance, skip: Ref, ids: &mut HashMap<Ref, usize>) {
            ids.insert(i.referent(), ids.len() + 1);
            for r in i.children() {
                if *r != skip {
                    visit(dom, dom.get_by_ref(*r).unwrap(), skip, ids);
                }
            }
        }
        let skip = if mesh.class == "SpecialMesh" {
            mesh.referent()
        } else {
            Ref::none()
        };
        visit(&self.dom, owner, skip, &mut ids);
        let mut refs = Vec::new();
        fn node(
            dom: &WeakDom,
            i: &Instance,
            skip: Ref,
            ids: &HashMap<Ref, usize>,
            refs: &mut Vec<Value>,
            root: bool,
        ) -> Result<Value> {
            let mut properties = BTreeMap::new();
            let mut attributes = BTreeMap::new();
            for (key, value) in &i.properties {
                let key = key.as_str();
                // Identity and engine-generated geometry blobs must not be carried
                // onto a freshly generated mesh. User data is retained.
                if matches!(
                    key,
                    "UniqueId"
                        | "HistoryId"
                        | "SourceAssetId"
                        | "ScriptGuid"
                        | "Capabilities"
                        | "DefinesCapabilities"
                ) || (root
                    && matches!(
                        key,
                        "MeshId"
                            | "MeshContent"
                            | "MeshSize"
                            | "InitialSize"
                            | "PhysicsData"
                            | "PhysicalConfigData"
                            | "HasSkinnedMesh"
                            | "VertexCount"
                            | "MeshVersion"
                    ))
                {
                    continue;
                }
                if let Variant::Ref(r) = value {
                    if r.is_some() {
                        let target = ids.get(r).with_context(|| {
                            format!("{}.{} references outside the mesh subtree", i.name, key)
                        })?;
                        refs.push(
                            json!({"Source":ids[&i.referent()],"Property":key,"Target":target}),
                        );
                    }
                } else if let Variant::Attributes(a) = value {
                    for (k, v) in a.iter() {
                        ensure!(!k.starts_with("__RMUS"), "reserved attribute prefix __RMUS");
                        attributes.insert(k.clone(), serde_json::to_value(v)?);
                    }
                } else {
                    let encoded = match value {
                        Variant::Content(v) => {
                            if v.as_uri().is_some() || v == &rbx_dom_weak::types::Content::none() {
                                serde_json::to_value(value)?
                            } else {
                                bail!(
                                    "object-backed content on {}.{} must be published before export",
                                    i.name,
                                    key
                                );
                            }
                        }
                        _ => serde_json::to_value(value)?,
                    };
                    properties.insert(key.to_owned(), encoded);
                }
            }
            attributes.insert("__RMUSNode".into(), json!({"Float64":ids[&i.referent()]}));
            let children = i
                .children()
                .iter()
                .filter(|r| **r != skip)
                .map(|r| node(dom, dom.get_by_ref(*r).unwrap(), skip, ids, refs, false))
                .collect::<Result<Vec<_>>>()?;
            Ok(
                json!({"className":if root {"MeshPart"} else {i.class.as_str()},"name":if root {"Template"} else {&i.name},"properties":properties,"attributes":attributes,"children":children}),
            )
        }
        let template = node(&self.dom, owner, skip, &ids, &mut refs, true)?;
        Ok(
            json!({"className":"Configuration","attributes":{"__RMUSVersion":1,"__RMUSReferences":serde_json::to_string(&refs)?},"children":[template]}),
        )
    }
}
