use std::{error::Error, f32::consts::PI, path::Path};

use bevy::{
    log::Level,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use voxel_engine_prototype_lib::{
    camera_move_system::{camera_move_system, CameraMoveSensitivity},
    game_config::{GameConfig, GameConfigPlugin},
    voxels::systems::{
        dirty_around_system::RenderAround, generate_map_around_system::GenerateMapAround,
        materials::Materials, VoxelBundle,
    },
};

fn main() -> Result<(), Box<dyn Error>> {
    // let mut game_data = DispatcherBuilder::default();
    // game_data
    //     .add_bundle(LoaderBundle)
    //     .add_bundle(VoxelBundle::default())
    let config_path = Path::new("config");

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            level: Level::INFO,
            ..default()
        }))
        .add_plugin(GameConfigPlugin::new(GameConfig::from_file_ron(
            config_path.join("game_configs.ron"),
        )?))
        .add_plugin(VoxelBundle)
        .add_startup_system(startup)
        .add_startup_system(add_camera)
        .add_system(camera_move_system)
        .run();

    Ok(())
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 14.5;

fn add_camera(mut commands: Commands) {
    commands.insert_resource(CameraMoveSensitivity::default());
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    commands.insert_resource(Materials {
        material: debug_material.clone(),
    });

    let shapes = [
        meshes.add(shape::Cube::default().into()),
        meshes.add(shape::Box::default().into()),
        meshes.add(shape::Capsule::default().into()),
        meshes.add(shape::Torus::default().into()),
        meshes.add(shape::Cylinder::default().into()),
        meshes.add(shape::Icosphere::default().try_into().unwrap()),
        meshes.add(shape::UVSphere::default().into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    2.0,
                    0.0,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 6., 12.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            ..default()
        })
        .insert((RenderAround, GenerateMapAround));
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    )
}
