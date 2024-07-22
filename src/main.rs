use avian2d::{math::Vector, prelude::*};
use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use character_controller::{CharacterControllerBundle, CharacterControllerPlugin};

mod character_controller;
mod level0;

#[derive(Default, Resource)]
struct SpikeData {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<ColorMaterial>>,
}

impl SpikeData {
    fn ensure_initialized(
        &mut self,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        if self.mesh.is_none() {
            self.mesh = Some(meshes.add(Triangle2d::new(
                Vec2::Y * 24.,
                Vec2::new(-12., 0.),
                Vec2::new(12., 0.),
            )));
        }
        if self.material.is_none() {
            self.material = Some(materials.add(Color::srgb(1., 0., 0.)));
        }
    }

    fn mesh(&self) -> Option<Handle<Mesh>> {
        self.mesh.as_ref().map(Handle::clone_weak)
    }

    fn material(&self) -> Option<Handle<ColorMaterial>> {
        self.material.as_ref().map(Handle::clone_weak)
    }
}

#[derive(Event)]
struct DeathEvent;

#[derive(Component)]
struct LevelRoot;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Spike;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // 1 meter = 20 pixels
            PhysicsPlugins::default().with_length_unit(20.),
        ))
        .add_plugins(CharacterControllerPlugin)
        .add_event::<DeathEvent>()
        .insert_resource(Gravity(Vector::NEG_Y * 1000.))
        .insert_resource(SpikeData::default())
        .add_systems(Startup, (setup, setup_level).chain())
        .add_systems(
            Update,
            (
                camera_smooth_follow_player,
                (rotate_spikes, death_condition, reset_level).chain(),
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(1., 0.7, 0.),
                custom_size: Some(Vec2::new(20., 40.)),
                ..default()
            },
            ..default()
        },
        Player,
        CharacterControllerBundle::new(Collider::rectangle(20., 40.)),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        ColliderDensity(2.),
    ));
}

fn setup_level(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    mut spike_data: ResMut<SpikeData>,
) {
    spike_data.ensure_initialized(meshes, materials);

    fn spawn_entities(parent: &mut ChildBuilder, spike_data: &SpikeData) {
        parent.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(spike_data.mesh().unwrap()),
                material: spike_data.material().unwrap(),
                transform: Transform::from_xyz(100., 0., 5.),
                ..default()
            },
            Spike,
            Collider::rectangle(24., 24.),
        ));

        level0::setup(parent);
    }

    commands
        .spawn((
            LevelRoot,
            TransformBundle::from_transform(Transform::from_xyz(0., -30., 0.)),
            VisibilityBundle::default(),
        ))
        .with_children(|commands| spawn_entities(commands, &*spike_data));
}

fn rotate_spikes(mut spikes: Query<&mut Transform, With<Spike>>) {
    for mut spike in &mut spikes {
        spike.rotate_z(0.1);
    }
}

fn camera_smooth_follow_player(
    mut cameras: Query<&mut Transform, With<Camera2d>>,
    player: Query<&Transform, (With<Player>, Without<Camera2d>)>,
) {
    let player = player.single();

    for mut camera in &mut cameras {
        camera.translation = camera.translation.lerp(player.translation, 0.1);
    }
}

fn death_condition(
    mut collision_event_reader: EventReader<Collision>,
    player: Query<Entity, With<Player>>,
    spikes: Query<Entity, With<Spike>>,
    mut death_event_writer: EventWriter<DeathEvent>,
) {
    let player = player.single();
    for Collision(contacts) in collision_event_reader.read() {
        let other_entity = if contacts.entity1 == player {
            contacts.entity2
        } else if contacts.entity2 == player {
            contacts.entity1
        } else {
            continue;
        };

        if !spikes.contains(other_entity) {
            continue;
        }

        death_event_writer.send(DeathEvent);
        return;
    }
}

fn reset_level(
    mut commands: Commands,
    mut death_event_reader: EventReader<DeathEvent>,
    level_root: Query<Entity, With<LevelRoot>>,
    mut player: Query<&mut Transform, With<Player>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    spike_data: ResMut<SpikeData>,
) {
    if death_event_reader.read().count() == 0 {
        return;
    }

    let level_root = level_root.single();
    commands.entity(level_root).despawn_recursive();

    let mut player = player.single_mut();
    player.translation = Vec3::ZERO;

    setup_level(commands, meshes, materials, spike_data);
}
