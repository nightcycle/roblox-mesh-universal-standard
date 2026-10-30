//! ASCII/binary FBX and Wavefront OBJ via ufbx.
use crate::schema::{Bone, Document, Face, Pose, Skin};
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;

fn v3(v: ufbx::Vec3) -> glam::Vec3 {
    glam::Vec3::new(v.x as f32, v.y as f32, v.z as f32)
}
fn mat(m: &ufbx::Matrix) -> glam::Mat4 {
    glam::Mat4::from_cols_array(&[
        m.m00 as f32,
        m.m10 as f32,
        m.m20 as f32,
        0.,
        m.m01 as f32,
        m.m11 as f32,
        m.m21 as f32,
        0.,
        m.m02 as f32,
        m.m12 as f32,
        m.m22 as f32,
        0.,
        m.m03 as f32,
        m.m13 as f32,
        m.m23 as f32,
        1.,
    ])
}
pub fn decode(bytes: &[u8], filename: &str, scale: f32) -> Result<Document> {
    ensure!(
        scale.is_finite() && scale > 0.,
        "scale must be positive and finite"
    );
    let scene = ufbx::load_memory(
        bytes,
        ufbx::LoadOpts {
            filename: filename.into(),
            load_external_files: false,
            ignore_embedded: true,
            generate_missing_normals: true,
            normalize_normals: true,
            target_axes: ufbx::CoordinateAxes {
                right: ufbx::CoordinateAxis::PositiveX,
                up: ufbx::CoordinateAxis::PositiveY,
                front: ufbx::CoordinateAxis::PositiveZ,
            },
            ..Default::default()
        },
    )
    .map_err(|e| anyhow::anyhow!("OBJ/FBX import: {e:?}"))?;
    let nodes: Vec<_> = scene
        .nodes
        .iter()
        .filter(|n| n.mesh.as_ref().is_some_and(|m| m.num_faces > 0))
        .collect();
    ensure!(
        nodes.len() == 1,
        "expected one mesh instance in OBJ/FBX; found {}",
        nodes.len()
    );
    let node = nodes[0];
    let mesh = node.mesh.as_ref().unwrap();
    ensure!(
        mesh.num_triangles <= 20000 && mesh.num_vertices <= 60000,
        "mesh exceeds EditableMesh limits"
    );
    ensure!(
        mesh.blend_deformers.is_empty(),
        "FBX vertex blend shapes are not representable by EditableMesh; use Roblox bone-based FACS poses"
    );
    ensure!(
        mesh.cache_deformers.is_empty(),
        "vertex cache animation is not representable by EditableMesh"
    );
    ensure!(
        mesh.skin_deformers.len() <= 1,
        "multiple FBX skin deformers are not supported by Roblox"
    );
    let transform = glam::Mat4::from_scale(glam::Vec3::splat(scale)) * mat(&node.geometry_to_world);
    ensure!(
        transform.determinant().abs() > 1e-12,
        "singular mesh transform"
    );
    let normal_transform = transform.inverse().transpose();
    let mut d = Document::default();
    d.metadata.insert(
        "sourceFormat".into(),
        if filename.to_lowercase().ends_with(".obj") {
            "obj"
        } else {
            "fbx"
        }
        .into(),
    );
    d.metadata
        .insert("sourceName".into(), node.element.name.to_string().into());
    d.metadata.insert("scale".into(), scale.into());
    for v in &mesh.vertices {
        d.vertices
            .push(transform.transform_point3(v3(*v)).to_array());
    }
    for n in &mesh.vertex_normal.values {
        d.normals.push(
            normal_transform
                .transform_vector3(v3(*n))
                .normalize()
                .to_array(),
        );
    }
    for uv in &mesh.vertex_uv.values {
        d.uvs.push([uv.x as f32, 1. - uv.y as f32]);
    }
    for c in &mesh.vertex_color.values {
        d.colors
            .push([c.x as f32, c.y as f32, c.z as f32, c.w as f32]);
    }
    let mut indices = Vec::new();
    for f in &mesh.faces {
        let count = ufbx::triangulate_face_vec(&mut indices, mesh, *f) as usize;
        for tri in indices[..count * 3].as_chunks::<3>().0 {
            let mut corners = tri.to_vec();
            if transform.determinant() < 0. {
                corners.swap(1, 2);
            }
            let attr = |exists: bool, values: &[u32]| -> Vec<usize> {
                if exists {
                    corners
                        .iter()
                        .map(|i| values[*i as usize] as usize + 1)
                        .collect()
                } else {
                    vec![]
                }
            };
            d.faces.push(Face {
                vertices: corners
                    .iter()
                    .map(|i| mesh.vertex_indices[*i as usize] as usize + 1)
                    .collect(),
                normals: attr(mesh.vertex_normal.exists, &mesh.vertex_normal.indices),
                uvs: attr(mesh.vertex_uv.exists, &mesh.vertex_uv.indices),
                colors: attr(mesh.vertex_color.exists, &mesh.vertex_color.indices),
            });
        }
    }
    if let Some(skin) = mesh.skin_deformers.first() {
        ensure!(
            skin.skinning_method == ufbx::SkinningMethod::Linear,
            "Roblox requires linear FBX skinning"
        );
        let mut nodes_by_id = HashMap::new();
        for c in &skin.clusters {
            let mut bone = c.bone_node.as_ref();
            while let Some(n) = bone {
                if n.is_root || n.mesh.is_some() {
                    break;
                }
                nodes_by_id.insert(n.element.element_id, n);
                bone = n.parent.as_ref();
            }
        }
        let mut bone_nodes: Vec<_> = nodes_by_id.values().copied().collect();
        bone_nodes.sort_by_key(|n| (n.node_depth, n.element.element_id));
        let mapping: HashMap<_, _> = bone_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.element.element_id, i + 1))
            .collect();
        for n in &bone_nodes {
            let bind = skin
                .clusters
                .iter()
                .find(|c| {
                    c.bone_node
                        .as_ref()
                        .is_some_and(|b| b.element.element_id == n.element.element_id)
                })
                .map_or(&n.node_to_world, |c| &c.bind_to_world);
            let (_, rotation, translation) = mat(bind).to_scale_rotation_translation();
            d.bones.push(Bone {
                name: n.element.name.to_string(),
                parent: n
                    .parent
                    .as_ref()
                    .and_then(|p| mapping.get(&p.element.element_id).copied()),
                is_virtual: false,
                cframe: crate::roblox::from_matrix(glam::Mat4::from_rotation_translation(
                    rotation,
                    translation * scale,
                )),
            });
        }
        for (i, v) in skin.vertices.iter().enumerate() {
            let mut bones = Vec::new();
            let mut weights = Vec::new();
            for w in &skin.weights.as_ref()
                [v.weight_begin as usize..(v.weight_begin + v.num_weights) as usize]
            {
                if w.weight <= 0. {
                    continue;
                }
                let node = skin.clusters[w.cluster_index as usize]
                    .bone_node
                    .as_ref()
                    .context("skin cluster without bone")?;
                bones.push(mapping[&node.element.element_id]);
                weights.push(w.weight as f32);
            }
            if !bones.is_empty() {
                let sum = weights.iter().sum::<f32>();
                for w in &mut weights {
                    *w /= sum;
                }
                d.skin.push(Skin {
                    vertex: i + 1,
                    bones,
                    weights,
                });
            }
        }
        d.require("skinning");
        let properties = &node.element.props.props;
        let frames = properties
            .iter()
            .filter_map(|p| {
                p.name
                    .as_ref()
                    .strip_prefix("Frame")
                    .and_then(|s| s.parse::<u32>().ok())
                    .map(|frame| (frame, p.value_str.to_string()))
            })
            .collect::<Vec<_>>();
        if frames.iter().any(|(_, name)| name != "Neutral") {
            let root_name = properties
                .iter()
                .find(|p| p.name == "RootFaceJoint")
                .context("FBX FACS frames require RootFaceJoint")?
                .value_str
                .as_ref();
            let root = bone_nodes
                .iter()
                .find(|n| n.element.name == root_name)
                .context("RootFaceJoint is not in the skin skeleton")?;
            ensure!(
                scene.anim_stacks.len() <= 1,
                "FACS FBX must have one animation take"
            );
            let fps = scene.settings.frames_per_second;
            ensure!(fps.is_finite() && fps > 0., "invalid FBX frame rate");
            let mut virtual_indices = Vec::new();
            for (i, n) in bone_nodes.iter().enumerate() {
                let mut current = n.parent.as_ref();
                while let Some(parent) = current {
                    if parent.element.element_id == root.element.element_id {
                        d.bones[i].is_virtual = true;
                        virtual_indices.push(i);
                        break;
                    }
                    current = parent.parent.as_ref();
                }
            }
            ensure!(
                !virtual_indices.is_empty(),
                "RootFaceJoint has no facial bones"
            );
            let neutral_frame = frames
                .iter()
                .find(|(_, name)| name == "Neutral")
                .context("FACS FBX requires a Neutral frame")?
                .0;
            let neutral = ufbx::evaluate_scene(
                &scene,
                &scene.anim,
                neutral_frame as f64 / fps,
                Default::default(),
            )
            .map_err(|e| anyhow::anyhow!("evaluating Neutral pose: {e:?}"))?;
            for &i in &virtual_indices {
                let id = bone_nodes[i].element.element_id;
                let n = neutral
                    .nodes
                    .iter()
                    .find(|n| n.element.element_id == id)
                    .context("missing neutral bone")?;
                let (_, q, t) = mat(&n.node_to_world).to_scale_rotation_translation();
                d.bones[i].cframe =
                    crate::roblox::from_matrix(glam::Mat4::from_rotation_translation(q, t * scale));
            }
            for (frame, name) in frames {
                if name == "Neutral" {
                    continue;
                }
                let actions = name.split('_').map(str::to_owned).collect::<Vec<_>>();
                ensure!(
                    (1..=3).contains(&actions.len()),
                    "invalid FBX corrective pose {name}"
                );
                let evaluated = ufbx::evaluate_scene(
                    &scene,
                    &scene.anim,
                    frame as f64 / fps,
                    Default::default(),
                )
                .map_err(|e| anyhow::anyhow!("evaluating FACS frame {frame}: {e:?}"))?;
                let mut cframes = Vec::new();
                for &i in &virtual_indices {
                    let id = bone_nodes[i].element.element_id;
                    let n = evaluated
                        .nodes
                        .iter()
                        .find(|n| n.element.element_id == id)
                        .context("missing posed bone")?;
                    let (_, q, t) = mat(&n.node_to_world).to_scale_rotation_translation();
                    let target = glam::Mat4::from_rotation_translation(q, t * scale);
                    cframes.push(crate::roblox::from_matrix(
                        target * crate::roblox::to_matrix(d.bones[i].cframe).inverse(),
                    ));
                }
                d.facs.push(Pose {
                    actions,
                    bones: virtual_indices.iter().map(|i| i + 1).collect(),
                    cframes,
                });
            }
            d.require("facs");
        }
    }
    if !d.normals.is_empty() {
        d.require("normals");
    }
    if !d.colors.is_empty() {
        d.require("colors");
    }
    if !d.uvs.is_empty() {
        d.require("uvs");
    }
    d.validate()?;
    Ok(d)
}
