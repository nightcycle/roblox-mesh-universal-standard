//! Roblox FileMesh v1-v7. Layout reference:
//! https://devforum.roblox.com/t/roblox-filemesh-format-specification/326114
use crate::schema::{Bone, Document, Face, Frame, Pose, Skin};
use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use binrw::{BinRead, BinReaderExt};
use rbx_mesh::mesh::*;
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

fn append(d: &mut Document, v: Vertex2) {
    d.vertices.push(v.pos);
    d.normals.push(v.norm);
    d.uvs.push(v.tex);
    d.colors.push(v.color.map(|c| c as f32 / 255.));
}
fn vertices(d: &mut Document, v: Vertices2) {
    match v {
        Vertices2::Full(v) => {
            for v in v {
                append(d, v);
            }
        }
        Vertices2::Truncated(v) => {
            for v in v {
                d.vertices.push(v.pos);
                d.normals.push(v.norm);
                d.uvs.push(v.tex);
                d.colors.push([1.; 4]);
            }
        }
    }
}
fn faces(d: &mut Document, fs: Vec<Face2>, lods: &[Lod3]) -> Result<()> {
    let end = lods.get(1).map_or(fs.len(), |l| l.0 as usize);
    let end = if end == 0 { fs.len() } else { end };
    ensure!(end <= fs.len(), "LOD range exceeds face count");
    d.metadata.insert("sourceFaceCount".into(), fs.len().into());
    d.metadata.insert(
        "lodOffsets".into(),
        json!(lods.iter().map(|l| l.0).collect::<Vec<_>>()),
    );
    for f in fs.into_iter().take(end) {
        let indices = f.0.map(|v| v.0 as usize + 1).to_vec();
        d.faces.push(Face {
            vertices: indices.clone(),
            normals: indices.clone(),
            colors: indices.clone(),
            uvs: indices,
        });
    }
    Ok(())
}
fn name(bytes: &[u8], offset: usize) -> Result<String> {
    let slice = bytes
        .get(offset..)
        .context("bone name offset out of bounds")?;
    let end = slice
        .iter()
        .position(|b| *b == 0)
        .context("unterminated bone name")?;
    Ok(std::str::from_utf8(&slice[..end])?.to_owned())
}
fn names(bytes: &[u8]) -> Result<Vec<String>> {
    if bytes.is_empty() {
        return Ok(vec![]);
    }
    ensure!(bytes.last() == Some(&0), "unterminated FACS names");
    bytes[..bytes.len() - 1]
        .split(|b| *b == 0)
        .map(|s| Ok(std::str::from_utf8(s)?.to_owned()))
        .collect()
}
fn skin(
    d: &mut Document,
    bones: Vec<Bone4>,
    bone_names: Vec<u8>,
    envelopes: Vec<Envelope4>,
    subsets: Vec<Subset4>,
) -> Result<()> {
    for b in bones {
        let c = b.cframe;
        d.bones.push(Bone {
            name: name(&bone_names, b.bone_name_pos as usize)?,
            parent: b.parent.get().map(|i| i as usize + 1),
            is_virtual: false,
            cframe: [
                c.x, c.y, c.z, c.r00, c.r01, c.r02, c.r10, c.r11, c.r12, c.r20, c.r21, c.r22,
            ],
        });
    }
    if d.bones.is_empty() {
        return Ok(());
    }
    ensure!(
        envelopes.len() == d.vertices.len(),
        "skinning vertex count mismatch"
    );
    let mut palettes = vec![None; envelopes.len()];
    for s in &subsets {
        ensure!(s.bone_count <= 26, "invalid subset palette");
        let start = s.vertices_offset as usize;
        let end = start + s.vertices_len as usize;
        ensure!(end <= envelopes.len(), "subset vertex range out of bounds");
        for slot in &mut palettes[start..end] {
            if let Some(previous) = slot {
                ensure!(*previous == &s.bones, "conflicting subset palettes");
            }
            *slot = Some(&s.bones);
        }
    }
    for (i, e) in envelopes.iter().enumerate() {
        let mut weights = BTreeMap::<usize, f32>::new();
        for j in 0..4 {
            if e.weights[j] == 0 {
                continue;
            }
            let palette = palettes[i].context("weighted vertex has no subset")?;
            let bone = palette
                .get(e.bones[j] as usize)
                .context("bone palette index out of bounds")?
                .get()
                .context("weight references empty bone")? as usize
                + 1;
            *weights.entry(bone).or_default() += e.weights[j] as f32;
        }
        if !weights.is_empty() {
            let sum = weights.values().sum::<f32>();
            d.skin.push(Skin {
                vertex: i + 1,
                bones: weights.keys().copied().collect(),
                weights: weights.values().map(|v| v / sum).collect(),
            });
        }
    }
    d.require("skinning");
    Ok(())
}
fn matrix(m: &QuantizedMatrix5) -> (usize, usize, Vec<f32>) {
    match m {
        QuantizedMatrix5::Raw { x, y, matrix } => (*x as usize, *y as usize, matrix.clone()),
        QuantizedMatrix5::Quantized {
            x,
            y,
            lerp0,
            lerp1,
            matrix,
        } => (
            *x as usize,
            *y as usize,
            matrix
                .iter()
                .map(|v| lerp0 + (*v as f32 / 65535.) * (lerp1 - lerp0))
                .collect(),
        ),
    }
}
pub fn from_matrix(m: glam::Mat4) -> Frame {
    let a = m.to_cols_array();
    [
        a[12], a[13], a[14], a[0], a[4], a[8], a[1], a[5], a[9], a[2], a[6], a[10],
    ]
}
pub fn to_matrix(c: Frame) -> glam::Mat4 {
    glam::Mat4::from_cols_array(&[
        c[3], c[6], c[9], 0., c[4], c[7], c[10], 0., c[5], c[8], c[11], 0., c[0], c[1], c[2], 1.,
    ])
}
fn facs(d: &mut Document, f: Facs5) -> Result<()> {
    let lookup: BTreeMap<String, String> = serde_json::from_str(include_str!("facs-names.json"))?;
    let bone_names = names(&f.face_bone_names)?;
    let controls = names(&f.face_control_names)?;
    let mut actions: Vec<Vec<String>> = controls
        .iter()
        .map(|c| vec![lookup.get(c).unwrap_or(c).clone()])
        .collect();
    for c in f.two_pose_correctives {
        actions.push(vec![
            actions.get(c.0.0 as usize).context("invalid corrective")?[0].clone(),
            actions.get(c.1.0 as usize).context("invalid corrective")?[0].clone(),
        ]);
    }
    for c in f.three_pose_correctives {
        actions.push(vec![
            actions.get(c.0.0 as usize).context("invalid corrective")?[0].clone(),
            actions.get(c.1.0 as usize).context("invalid corrective")?[0].clone(),
            actions.get(c.2.0 as usize).context("invalid corrective")?[0].clone(),
        ]);
    }
    let t = &f.quantized_transforms;
    let channels = [&t.px, &t.py, &t.pz, &t.rx, &t.ry, &t.rz].map(matrix);
    for (rows, cols, values) in &channels {
        ensure!(
            *rows == actions.len() && *cols == bone_names.len() && values.len() == rows * cols,
            "FACS channel dimensions mismatch"
        );
    }
    let bones = bone_names
        .iter()
        .map(|name| {
            let index = d
                .bones
                .iter()
                .position(|b| &b.name == name)
                .with_context(|| format!("missing FACS bone {name}"))?;
            d.bones[index].is_virtual = true;
            Ok(index + 1)
        })
        .collect::<Result<Vec<_>>>()?;
    // Files store corrective deltas. EditableMesh wants complete targets.
    // Sum matching base and subset corrective channels before Euler conversion.
    for (row, action) in actions.iter().enumerate() {
        let mut cframes = Vec::new();
        for col in 0..bones.len() {
            let mut v = [0f32; 6];
            for (other, subset) in actions.iter().enumerate().take(row + 1) {
                if subset.iter().all(|a| action.contains(a)) {
                    for ch in 0..6 {
                        v[ch] += channels[ch].2[other * bones.len() + col];
                    }
                }
            }
            let rotation = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                v[3].to_radians(),
                v[4].to_radians(),
                v[5].to_radians(),
            );
            let bind = to_matrix(d.bones[bones[col] - 1].cframe);
            let local =
                glam::Mat4::from_rotation_translation(rotation, glam::Vec3::new(v[0], v[1], v[2]));
            cframes.push(from_matrix(bind * local * bind.inverse()));
        }
        d.facs.push(Pose {
            actions: action.clone(),
            bones: bones.clone(),
            cframes,
        });
    }
    if !d.facs.is_empty() {
        d.require("facs");
    }
    Ok(())
}
fn read_vec<T>(r: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<T>>
where
    T: for<'a> BinRead<Args<'a> = ()>,
{
    ensure!(
        count <= 1_000_000 && count <= r.get_ref().len(),
        "unreasonable mesh element count"
    );
    (0..count).map(|_| Ok(r.read_le()?)).collect()
}
fn count(r: &mut Cursor<&[u8]>) -> Result<usize> {
    Ok(r.read_le::<u32>()? as usize)
}
fn bytes(r: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<u8>> {
    ensure!(
        count <= r.get_ref().len().saturating_sub(r.position() as usize),
        "truncated mesh"
    );
    let mut b = vec![0; count];
    r.read_exact(&mut b)?;
    Ok(b)
}
fn chunks(data: &[u8], d: &mut Document) -> Result<()> {
    let mut r = Cursor::new(&data[13..]);
    let mut chunks = BTreeMap::new();
    while (r.position() as usize) < r.get_ref().len() {
        let tag = bytes(&mut r, 8)?;
        let tag = std::str::from_utf8(&tag)?.trim_end_matches('\0').to_owned();
        let version = r.read_le::<u32>()?;
        let size = count(&mut r)?;
        let data = bytes(&mut r, size)?;
        ensure!(
            chunks.insert(tag.clone(), (version, data)).is_none(),
            "duplicate chunk {tag}"
        );
    }
    let (version, core) = chunks.remove("COREMESH").context("missing COREMESH")?;
    let mut r = Cursor::new(core.as_slice());
    let fs = match version {
        1 => {
            let n = count(&mut r)?;
            for v in read_vec::<Vertex2>(&mut r, n)? {
                append(d, v);
            }
            let n = count(&mut r)?;
            read_vec::<Face2>(&mut r, n)?
        }
        2 => {
            let n = count(&mut r)?;
            let stream = bytes(&mut r, n)?;
            draco(d, &stream)?
        }
        _ => bail!("unsupported COREMESH version {version}"),
    };
    ensure!(
        r.position() as usize == core.len(),
        "trailing COREMESH data"
    );
    let mut lods = vec![];
    if let Some((version, data)) = chunks.remove("LODS") {
        ensure!(version == 1, "unsupported LODS version");
        let mut r = Cursor::new(data.as_slice());
        r.set_position(3);
        let n = count(&mut r)?;
        lods = read_vec::<Lod3>(&mut r, n)?;
    }
    faces(d, fs, &lods)?;
    if let Some((version, data)) = chunks.remove("SKINNING") {
        ensure!(version == 1, "unsupported SKINNING version");
        let mut r = Cursor::new(data.as_slice());
        let n = count(&mut r)?;
        let envelopes = read_vec::<Envelope4>(&mut r, n)?;
        let n = count(&mut r)?;
        let bones = read_vec::<Bone4>(&mut r, n)?;
        let n = count(&mut r)?;
        let names = bytes(&mut r, n)?;
        let n = count(&mut r)?;
        let subsets = read_vec::<Subset4>(&mut r, n)?;
        skin(d, bones, names, envelopes, subsets)?;
    }
    if let Some((version, data)) = chunks.remove("FACS") {
        ensure!(version == 1, "unsupported FACS version");
        let mut r = Cursor::new(data.as_slice());
        let n = count(&mut r)?;
        ensure!(n == data.len() - 4, "FACS chunk size mismatch");
        if n > 0 {
            facs(d, r.read_le()?)?;
        }
    }
    for (tag, (version, data)) in chunks {
        d.extensions.insert(format!("roblox.chunk.{tag}"),json!({"Version":version,"Base64":base64::engine::general_purpose::STANDARD.encode(data)}));
    }
    Ok(())
}
pub fn decode(data: &[u8]) -> Result<Document> {
    let mut d = Document::default();
    ensure!(data.len() > 13, "truncated mesh");
    if data.starts_with(b"version 6.00\n") || data.starts_with(b"version 7.00\n") {
        chunks(data, &mut d)?;
    } else if data.starts_with(b"version 5.00\n") {
        // v5 permits absent FACS; rbx_mesh's v5 parser assumes it exists.
        ensure!(data.len() >= 45, "truncated v5 header");
        let format = u32::from_le_bytes(data[37..41].try_into()?);
        let size = u32::from_le_bytes(data[41..45].try_into()?) as usize;
        ensure!(size <= data.len() - 45, "invalid FACS size");
        let mut v4 = b"version 4.00\n".to_vec();
        v4.extend_from_slice(&24u16.to_le_bytes());
        v4.extend_from_slice(&data[15..37]);
        v4.extend_from_slice(&data[45..data.len() - size]);
        d = decode(&v4)?;
        if size > 0 {
            ensure!(format == 1, "unknown FACS format {format}");
            facs(&mut d, Cursor::new(&data[data.len() - size..]).read_le()?)?;
        }
    } else {
        match rbx_mesh::read_mesh_versioned(Cursor::new(data))
            .context("unsupported or malformed Roblox mesh version")?
        {
            Mesh::V1(m) => {
                for v in m.vertices {
                    d.vertices.push(v.pos);
                    d.normals.push(v.norm);
                    d.uvs.push([v.tex[0], v.tex[1]]);
                    d.colors.push([1.; 4]);
                }
                for i in 0..d.vertices.len() / 3 {
                    let v = vec![3 * i + 1, 3 * i + 2, 3 * i + 3];
                    d.faces.push(Face {
                        vertices: v.clone(),
                        normals: v.clone(),
                        colors: v.clone(),
                        uvs: v,
                    });
                }
            }
            Mesh::V2(m) => {
                vertices(&mut d, m.vertices);
                faces(&mut d, m.faces, &[])?;
            }
            Mesh::V3(m) => {
                vertices(&mut d, m.vertices);
                faces(&mut d, m.faces, &m.lods)?;
            }
            Mesh::V4(m) => {
                for v in m.vertices {
                    append(&mut d, v);
                }
                faces(&mut d, m.faces, &m.lods)?;
                skin(&mut d, m.bones, m.bone_names, m.envelopes, m.subsets)?;
            }
            Mesh::V5(_) => unreachable!(),
        }
    }
    d.metadata.insert(
        "robloxMeshVersion".into(),
        String::from_utf8_lossy(&data[..12]).trim().into(),
    );
    for f in ["normals", "colors", "uvs"] {
        d.require(f);
    }
    d.validate()?;
    Ok(d)
}

fn draco(d: &mut Document, bytes: &[u8]) -> Result<Vec<Face2>> {
    use draco_core::{DecoderBuffer, Mesh as DracoMesh, MeshDecoder};
    let mut mesh = DracoMesh::new();
    MeshDecoder::new().decode(&mut DecoderBuffer::new(bytes), &mut mesh)?;
    let pc = &mesh;
    ensure!(pc.num_points() <= 60000, "Draco mesh exceeds vertex limit");
    for i in 0..pc.num_points() {
        let read = |id: u32, components: usize, byte: bool| -> Result<Vec<f32>> {
            let a = pc
                .attribute_by_unique_id(id)
                .with_context(|| format!("Draco missing attribute {id}"))?;
            ensure!(
                a.num_components() as usize == components,
                "Draco component mismatch"
            );
            let index = a.mapped_index(draco_core::PointIndex(i as u32));
            let offset = index.0 as usize * a.byte_stride() as usize;
            let data = a.buffer().data();
            ensure!(
                a.data_type()
                    == if byte {
                        draco_core::DataType::Uint8
                    } else {
                        draco_core::DataType::Float32
                    },
                "unexpected Draco attribute type"
            );
            let size = components * if byte { 1 } else { 4 };
            let data = data
                .get(offset..offset + size)
                .context("truncated Draco attribute")?;
            if byte {
                Ok(data.iter().map(|v| *v as f32 / 255.).collect())
            } else {
                Ok(data
                    .chunks_exact(4)
                    .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
                    .collect())
            }
        };
        d.vertices.push(read(0, 3, false)?.try_into().unwrap());
        d.normals.push(read(1, 3, false)?.try_into().unwrap());
        d.uvs.push(read(2, 2, false)?.try_into().unwrap());
        d.colors.push(if pc.attribute_by_unique_id(4).is_some() {
            read(4, 4, true)?.try_into().unwrap()
        } else {
            [1.; 4]
        });
    }
    Ok(mesh
        .faces()
        .iter()
        .map(|f| Face2(f.map(|i| VertexId2(i.0))))
        .collect())
}
