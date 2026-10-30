use rbx_dom_weak::{
    InstanceBuilder, WeakDom,
    types::{CFrame, Content, Matrix3, Vector3},
};
use roblox_mesh_universal_standard::{
    assets::{Assets, asset_id},
    exchange,
    model::Model,
    roblox,
    schema::Document,
};
use std::fs;

fn f32s(out: &mut Vec<u8>, values: &[f32]) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
fn u32s(out: &mut Vec<u8>, values: &[u32]) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
fn vertex(out: &mut Vec<u8>, p: [f32; 3]) {
    f32s(out, &p);
    f32s(out, &[0., 0., 1., 0., 0.]);
    out.extend([0, 0, 0, 0]);
    out.extend([255, 128, 0, 64]);
}
fn core() -> Vec<u8> {
    let mut b = vec![];
    u32s(&mut b, &[3]);
    for p in [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        vertex(&mut b, p);
    }
    u32s(&mut b, &[1, 0, 1, 2]);
    b
}
fn chunk(b: &mut Vec<u8>, tag: &str, version: u32, data: &[u8]) {
    let mut name = [0; 8];
    name[..tag.len()].copy_from_slice(tag.as_bytes());
    b.extend(name);
    u32s(b, &[version, data.len() as u32]);
    b.extend(data);
}
fn sample() -> Document {
    let mut b = b"version 6.00\n".to_vec();
    chunk(&mut b, "COREMESH", 1, &core());
    roblox::decode(&b).unwrap()
}
#[test]
fn ascii_v1_applies_historical_scale_and_uv_flip() {
    let b = b"version 1.00\n1\n[0,0,0][0,0,1][0,0,0][2,0,0][0,0,1][1,0,0][0,2,0][0,0,1][0,1,0]\n";
    let d = roblox::decode(b).unwrap();
    assert_eq!(d.vertices[1], [1., 0., 0.]);
    assert_eq!(d.uvs[0], [0., 1.]);
    assert_eq!(d.faces[0].vertices, vec![1, 2, 3]);
}
#[test]
fn chunked_v6_preserves_colors_and_unknown_chunks() {
    let mut b = b"version 6.00\n".to_vec();
    chunk(&mut b, "HSRAVIS", 1, &[1, 0, 0, 0, 1]);
    chunk(&mut b, "COREMESH", 1, &core());
    let d = roblox::decode(&b).unwrap();
    assert!((d.colors[0][3] - 64. / 255.).abs() < 1e-6);
    assert!(d.extensions.contains_key("roblox.chunk.HSRAVIS"));
    b.pop();
    assert!(roblox::decode(&b).is_err());
}
#[test]
fn v2_vertex_stride_and_v3_lod_range() {
    let mut b = b"version 2.00\n".to_vec();
    b.extend(12u16.to_le_bytes());
    b.extend([40, 12]);
    u32s(&mut b, &[3, 1]);
    for p in [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        vertex(&mut b, p);
    }
    u32s(&mut b, &[0, 1, 2]);
    assert_eq!(roblox::decode(&b).unwrap().faces.len(), 1);
    let mut b = b"version 3.00\n".to_vec();
    b.extend(16u16.to_le_bytes());
    b.extend([40, 12]);
    b.extend(4u16.to_le_bytes());
    b.extend(3u16.to_le_bytes());
    u32s(&mut b, &[3, 2]);
    for p in [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        vertex(&mut b, p);
    }
    u32s(&mut b, &[0, 1, 2, 0, 2, 1, 0, 1, 2]);
    assert_eq!(roblox::decode(&b).unwrap().faces.len(), 1);
}
fn skinned_v4() -> Vec<u8> {
    let mut b = b"version 4.00\n".to_vec();
    b.extend(24u16.to_le_bytes());
    b.extend(0u16.to_le_bytes());
    u32s(&mut b, &[3, 1]);
    b.extend(0u16.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    u32s(&mut b, &[10]);
    b.extend(1u16.to_le_bytes());
    b.extend([0, 0]);
    for p in [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        vertex(&mut b, p);
    }
    for _ in 0..3 {
        b.extend([0, 1, 0, 0, 64, 191, 0, 0]);
    }
    u32s(&mut b, &[0, 1, 2]);
    for (name, parent) in [(0, u16::MAX), (5, 0)] {
        u32s(&mut b, &[name]);
        b.extend(parent.to_le_bytes());
        b.extend(u16::MAX.to_le_bytes());
        f32s(
            &mut b,
            &[0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.],
        );
    }
    b.extend(b"Root\0Face\0");
    u32s(&mut b, &[0, 1, 0, 3, 2]);
    // Reverse palette proves local palette indices are not global bone indices.
    b.extend(1u16.to_le_bytes());
    b.extend(0u16.to_le_bytes());
    for _ in 0..24 {
        b.extend(u16::MAX.to_le_bytes());
    }
    b
}
#[test]
fn skinning_uses_subset_palette() {
    let d = roblox::decode(&skinned_v4()).unwrap();
    assert_eq!(d.bones[1].parent, Some(1));
    assert_eq!(d.skin[0].bones, vec![1, 2]);
    assert!((d.skin[0].weights[0] - 191. / 255.).abs() < 1e-6);
}
#[test]
fn v5_without_facs_is_valid() {
    let v4 = skinned_v4();
    let mut v5 = b"version 5.00\n".to_vec();
    v5.extend(32u16.to_le_bytes());
    v5.extend(&v4[15..37]);
    u32s(&mut v5, &[0, 0]);
    v5.extend(&v4[37..]);
    assert_eq!(roblox::decode(&v5).unwrap().bones.len(), 2);
}
#[test]
fn v5_facs_correctives_are_full_targets() {
    let mut facs = vec![];
    u32s(&mut facs, &[5, 10]);
    facs.extend(132u64.to_le_bytes());
    u32s(&mut facs, &[4, 0]);
    facs.extend(b"Face\0c_JD\0c_FN\0");
    for values in [[1., 2., 3.], [0.; 3], [0.; 3], [0.; 3], [0.; 3], [0.; 3]] {
        facs.extend(1u16.to_le_bytes());
        u32s(&mut facs, &[3, 1]);
        f32s(&mut facs, &values);
    }
    facs.extend(0u16.to_le_bytes());
    facs.extend(1u16.to_le_bytes());
    let v4 = skinned_v4();
    let mut v5 = b"version 5.00\n".to_vec();
    v5.extend(32u16.to_le_bytes());
    v5.extend(&v4[15..37]);
    u32s(&mut v5, &[1, facs.len() as u32]);
    v5.extend(&v4[37..]);
    v5.extend(facs);
    let d = roblox::decode(&v5).unwrap();
    assert!(d.bones[1].is_virtual);
    assert_eq!(d.facs[2].actions, vec!["JawDrop", "Funneler"]);
    assert_eq!(d.facs[2].cframes[0][0], 6.);
}
#[test]
fn obj_triangulates_concave_polygon_preserving_corner_normals() {
    let b=b"o Concave\nv 0 0 0 1 0 0\nv 2 0 0 0 1 0\nv 2 2 0 0 0 1\nv 1 1 0 1 1 0\nv 0 2 0 1 1 1\nvn 0 0 1\nf -5//1 -4//1 -3//1 -2//1 -1//1\n";
    let d = exchange::decode(b, "test.obj", 2.).unwrap();
    assert_eq!(d.faces.len(), 3);
    assert_eq!(d.vertices[1], [4., 0., 0.]);
    assert!(!d.colors.is_empty());
}
#[test]
fn schema_rejects_invalid_data_and_accepts_optional_extensions() {
    let d = sample();
    let mut invalid = d.clone();
    invalid.faces[0].vertices[0] = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = d.clone();
    invalid.colors[0][3] = f32::NAN;
    assert!(invalid.validate().is_err());
    let mut invalid = d.clone();
    invalid.version.major = 2;
    assert!(invalid.validate().is_err());
    let mut future = d.clone();
    future.version.minor = 42;
    future
        .extensions
        .insert("vendor.future".into(), true.into());
    future.validate().unwrap();
    future.required_features.push("vendor.future".into());
    assert!(future.validate().is_err());
    let encoded = serde_json::to_vec(&d).unwrap();
    let decoded: Document = serde_json::from_slice(&encoded).unwrap();
    decoded.validate().unwrap();
    let mut encoded = serde_json::to_value(&d).unwrap();
    encoded["Metadata"] = serde_json::json!([]);
    encoded["Extensions"] = serde_json::json!([]);
    let from_roblox: Document = serde_json::from_value(encoded).unwrap();
    from_roblox.validate().unwrap();
}
#[test]
fn rbxm_single_mesh_and_sidecar_references() {
    let mut dom = WeakDom::new(InstanceBuilder::new("DataModel"));
    let root = dom.root_ref();
    let mesh = dom.insert(
        root,
        InstanceBuilder::new("MeshPart")
            .with_name("Mesh")
            .with_property("MeshContent", Content::from_uri("rbxassetid://123"))
            .with_property("Size", Vector3::new(2., 3., 4.)),
    );
    let attachment = dom.insert(
        mesh,
        InstanceBuilder::new("Attachment")
            .with_name("Tip")
            .with_property(
                "CFrame",
                CFrame::new(Vector3::new(0., 2., 0.), Matrix3::identity()),
            ),
    );
    dom.insert(
        mesh,
        InstanceBuilder::new("ObjectValue")
            .with_name("Reference")
            .with_property("Value", attachment),
    );
    dom.insert(
        mesh,
        InstanceBuilder::new("SurfaceAppearance")
            .with_property("ColorMapContent", Content::from_uri("rbxassetid://456")),
    );
    let mut bytes = vec![];
    rbx_binary::to_writer(&mut bytes, &dom, &[mesh]).unwrap();
    let m = Model::read(&bytes).unwrap();
    assert_eq!(m.mesh().unwrap().content, "rbxassetid://123");
    let sidecar = m.sidecar().unwrap();
    assert_eq!(sidecar["className"], "Configuration");
    assert_eq!(
        sidecar["children"][0]["children"].as_array().unwrap().len(),
        3
    );
    let refs: serde_json::Value =
        serde_json::from_str(sidecar["attributes"]["__RMUSReferences"].as_str().unwrap()).unwrap();
    assert_eq!(refs[0]["Property"], "Value");
    assert!(
        sidecar["children"][0]["properties"]
            .get("MeshContent")
            .is_none()
    );
    dom.insert(root, InstanceBuilder::new("MeshPart"));
    assert!(Model { dom }.mesh().is_err());
}
#[test]
fn supplied_models_have_exactly_one_mesh_and_valid_exports() {
    for name in ["grass", "lilypad", "pillar"] {
        let m = Model::read(&fs::read(format!("test/{name}.rbxm")).unwrap()).unwrap();
        assert_eq!(m.mesh_instances().len(), 1);
        m.mesh().unwrap();
        let d: Document =
            serde_json::from_slice(&fs::read(format!("demo/assets/{name}.json")).unwrap()).unwrap();
        d.validate().unwrap();
    }
    assert!(!std::path::Path::new("test/tree.rbxm").exists());
}
#[test]
fn asset_ids_are_restricted_and_offline_maps_are_relative() {
    assert_eq!(asset_id("rbxassetid://123"), Some(123));
    assert_eq!(asset_id("https://www.roblox.com/asset/?id=123"), Some(123));
    assert_eq!(asset_id("https://evil.example/?id=123"), None);
    assert_eq!(asset_id("rbxassetid://0"), None);
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("mesh.bin"), b"test").unwrap();
    let map = tmp.path().join("map.json");
    fs::write(&map, br#"{"rbxassetid://1":"mesh.bin"}"#).unwrap();
    let assets = Assets::new(tmp.path().join("cache"), true, Some(&map)).unwrap();
    assert_eq!(assets.load("rbxassetid://1").unwrap(), b"test");
    assert!(assets.load("rbxassetid://2").is_err());
}
#[test]
fn ascii_binary_and_skinned_fbx_import() {
    for name in [
        "blender_272_cube_7400_binary.fbx",
        "blender_279_default_6100_ascii.fbx",
        "blender_293_half_skinned_7400_binary.fbx",
        "maya_advanced_skinned_pivot_7700_ascii.fbx",
    ] {
        let d = exchange::decode(
            &fs::read(format!("test/fixtures/{name}")).unwrap(),
            name,
            1.,
        )
        .unwrap();
        d.validate().unwrap();
        if name.contains("skinned") {
            assert!(!d.bones.is_empty() && !d.skin.is_empty());
        } else {
            assert_eq!(d.faces.len(), 12);
        }
    }
}
#[test]
fn draco_v7_decodes_attributes_by_unique_id() {
    use draco_core::{
        DataType, EncoderBuffer, EncoderOptions, GeometryAttributeType as Kind, Mesh, MeshEncoder,
        PointAttribute, PointIndex,
    };
    let mut mesh = Mesh::new();
    mesh.set_num_points(3);
    for (id, kind, components, values) in [
        (
            0,
            Kind::Position,
            3,
            vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
        ),
        (
            1,
            Kind::Generic,
            3,
            vec![0., 0., 1., 0., 0., 1., 0., 0., 1.],
        ),
        (2, Kind::TexCoord, 2, vec![0., 0., 1., 0., 0., 1.]),
    ] {
        let mut a = PointAttribute::new();
        a.init(kind, components, DataType::Float32, false, 3);
        a.set_unique_id(id);
        let bytes: Vec<u8> = values.iter().flat_map(|v: &f32| v.to_le_bytes()).collect();
        a.buffer_mut().update(&bytes, Some(0));
        mesh.add_attribute_preserve_unique_id(a);
    }
    let mut a = PointAttribute::new();
    a.init(Kind::Color, 4, DataType::Uint8, false, 3);
    a.set_unique_id(4);
    a.buffer_mut()
        .update(&[255, 0, 0, 64, 0, 255, 0, 128, 0, 0, 255, 255], Some(0));
    mesh.add_attribute_preserve_unique_id(a);
    mesh.add_face([PointIndex(0), PointIndex(1), PointIndex(2)]);
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);
    let mut out = EncoderBuffer::new();
    encoder.encode(&EncoderOptions::new(), &mut out).unwrap();
    let mut data = vec![];
    u32s(&mut data, &[out.data().len() as u32]);
    data.extend(out.data());
    let mut b = b"version 7.00\n".to_vec();
    chunk(&mut b, "COREMESH", 2, &data);
    let d = roblox::decode(&b).unwrap();
    assert_eq!(d.faces.len(), 1);
    assert_eq!(d.vertices.len(), 3);
    assert!(d.colors.iter().any(|c| (c[3] - 64. / 255.).abs() < 1e-6));
}
#[test]
fn roblox_fbx_frame_properties_become_facs_targets() {
    let source =
        fs::read_to_string("test/fixtures/maya_advanced_skinned_pivot_7700_ascii.fbx").unwrap();
    let marker = "P: \"currentUVSet\", \"KString\", \"\", \"U\", \"map1\"";
    assert!(source.contains(marker));
    let data=source.replacen(marker,&format!("{marker}\nP: \"RootFaceJoint\", \"KString\", \"\", \"U\", \"joint1\"\nP: \"Frame0\", \"KString\", \"\", \"U\", \"Neutral\"\nP: \"Frame1\", \"KString\", \"\", \"U\", \"JawDrop\"\nP: \"Frame2\", \"KString\", \"\", \"U\", \"Funneler\"\nP: \"Frame3\", \"KString\", \"\", \"U\", \"JawDrop_Funneler\""),1);
    let d = exchange::decode(data.as_bytes(), "head.fbx", 1.).unwrap();
    assert_eq!(d.facs.len(), 3);
    assert!(d.bones.iter().any(|b| b.is_virtual));
    assert!(d.facs.iter().any(|p| p.actions.len() == 2));
    assert!(
        d.facs
            .iter()
            .flat_map(|p| &p.cframes)
            .all(roblox_mesh_universal_standard::schema::valid_frame)
    );
}
