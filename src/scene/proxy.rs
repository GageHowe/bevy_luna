use super::{DisableRaytracingMesh, RaytracingMesh3d};
use bevy::{
    asset::AssetId,
    mesh::{Indices, Mesh, Mesh3d, MeshVertexAttributeId, VertexAttributeValues},
    pbr::MeshMaterial3d,
    platform::collections::HashMap,
    prelude::*,
};

#[derive(Resource, Default)]
pub struct RaytraceProxyMeshes(pub HashMap<AssetId<Mesh>, Handle<Mesh>>);

pub fn tag_raytracing_meshes(
    mut commands: Commands,
    mut meshes_assets: ResMut<Assets<Mesh>>,
    proxy_meshes: Option<ResMut<RaytraceProxyMeshes>>,
    meshes: Query<
        (Entity, &Mesh3d),
        (
            With<MeshMaterial3d<StandardMaterial>>,
            Without<RaytracingMesh3d>,
            Without<DisableRaytracingMesh>,
        ),
    >,
) {
    let proxy_meshes = proxy_meshes;
    if proxy_meshes.is_none() {
        commands.init_resource::<RaytraceProxyMeshes>();
        return;
    }
    let mut proxy_meshes = proxy_meshes.expect("checked is_some above");

    for (entity, mesh) in &meshes {
        let source_id = mesh.id();
        if let Some(proxy) = proxy_meshes.0.get(&source_id) {
            commands
                .entity(entity)
                .insert(RaytracingMesh3d(proxy.clone()));
            continue;
        }

        let Some(source_mesh) = meshes_assets.get(&mesh.0) else {
            continue;
        };
        let Some(proxy_mesh) = make_raytracing_proxy_mesh(source_mesh) else {
            continue;
        };

        let proxy = meshes_assets.add(proxy_mesh);
        proxy_meshes.0.insert(source_id, proxy.clone());
        commands.entity(entity).insert(RaytracingMesh3d(proxy));
    }
}

pub fn make_raytracing_proxy_mesh(source: &Mesh) -> Option<Mesh> {
    if source.primitive_topology() != bevy::render::render_resource::PrimitiveTopology::TriangleList
    {
        return None;
    }

    let positions = clone_attribute(source, Mesh::ATTRIBUTE_POSITION.id)?;
    let normals = clone_attribute(source, Mesh::ATTRIBUTE_NORMAL.id)?;
    let uvs = clone_attribute(source, Mesh::ATTRIBUTE_UV_0.id)?;
    let indices = match source.indices()? {
        Indices::U16(values) => {
            Indices::U32(values.iter().map(|&index| u32::from(index)).collect())
        }
        Indices::U32(values) => Indices::U32(values.clone()),
    };

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        source.asset_usage,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(indices);

    if let Some(tangents) = clone_attribute(source, Mesh::ATTRIBUTE_TANGENT.id) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    } else if mesh.generate_tangents().is_err() {
        return None;
    }

    mesh.enable_raytracing = true;
    Some(mesh)
}

fn clone_attribute(
    source: &Mesh,
    attribute: MeshVertexAttributeId,
) -> Option<VertexAttributeValues> {
    source.attribute(attribute).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;

    #[test]
    fn proxy_mesh_enables_raytracing_and_upgrades_indices() {
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        );
        mesh.insert_indices(Indices::U16(vec![0, 1, 2]));

        let proxy = make_raytracing_proxy_mesh(&mesh).expect("proxy mesh should be generated");
        assert!(proxy.enable_raytracing);
        assert!(matches!(proxy.indices(), Some(Indices::U32(_))));
    }
}
