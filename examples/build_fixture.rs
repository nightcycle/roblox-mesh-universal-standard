//! Rebuild the small child-preservation fixture using the production exporter.
use rbx_dom_weak::{
    InstanceBuilder, WeakDom,
    types::{Attributes, CFrame, Content, Matrix3, Variant, Vector3},
};
fn main() -> anyhow::Result<()> {
    let mut dom = WeakDom::new(InstanceBuilder::new("DataModel"));
    let root = dom.root_ref();
    let mut attributes = Attributes::new();
    attributes.insert("Example".into(), Variant::String("preserved".into()));
    let mesh = dom.insert(
        root,
        InstanceBuilder::new("MeshPart")
            .with_name("Fixture")
            .with_property("MeshContent", Content::from_uri("rbxassetid://1"))
            .with_property("Size", Vector3::new(1., 1., 0.05))
            .with_property("Attributes", attributes),
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
    let bone = dom.insert(
        mesh,
        InstanceBuilder::new("Bone")
            .with_name("Root")
            .with_property(
                "CFrame",
                CFrame::new(Vector3::new(1., 0., 0.), Matrix3::identity()),
            ),
    );
    dom.insert(bone, InstanceBuilder::new("Bone").with_name("Child"));
    dom.insert(
        mesh,
        InstanceBuilder::new("ObjectValue")
            .with_name("TipReference")
            .with_property("Value", attachment),
    );
    dom.insert(
        mesh,
        InstanceBuilder::new("ObjectValue")
            .with_name("SelfReference")
            .with_property("Value", mesh),
    );
    dom.insert(
        mesh,
        InstanceBuilder::new("SurfaceAppearance").with_property(
            "ColorMapContent",
            Content::from_uri("rbxassetid://6341664856"),
        ),
    );
    let result = roblox_mesh_universal_standard::model::Model { dom }.sidecar()?;
    std::fs::write(
        "test/fixtures/children.model.json",
        serde_json::to_vec_pretty(&result)?,
    )?;
    Ok(())
}
