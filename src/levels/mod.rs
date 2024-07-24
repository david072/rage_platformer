use avian2d::{math::Vector, prelude::*};
use bevy::{
    color::palettes::css::*,
    ecs::system::EntityCommands,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use level0::Level0;
use level1::Level1;

mod level0;
mod level1;

const PLATFORM_Z: f32 = 10.;
const SPIKE_Z: f32 = 5.;
const DOOR_Z: f32 = -1.;
const LEVEL_TEXT_Z: f32 = -10.;
const SPIKE_SIZE: Vec2 = Vec2::new(24., 24.);
const PLATFORM_THICKNESS: f32 = 4.;
const DOOR_SIZE: Vec2 = Vec2::new(30., 50.);

#[derive(Component)]
pub struct LevelEnd;

#[derive(Component)]
pub struct Spike;

#[derive(Component)]
pub enum MovingPlatformType {
    Slider {
        a: Vec3,
        b: Vec3,
        speed: f32,
        delta_t_per_second: f32,
    },
}

impl MovingPlatformType {
    /// speed: u/s
    pub fn slider(a: Vec3, b: Vec3, speed: f32) -> Self {
        Self::Slider {
            a,
            b,
            speed,
            delta_t_per_second: speed / a.distance(b),
        }
    }
}

#[derive(Default, Component)]
pub struct MovingPlatform {
    pub active: bool,
    pub t: f32,
    pub moving_backward: bool,
}

#[derive(Bundle)]
struct PlatformBundle {
    sprite: SpriteBundle,
    collider: Collider,
    rigid_body: RigidBody,
}

impl PlatformBundle {
    pub fn new(pos: (f32, f32), size: f32) -> Self {
        let size = Self::make_size_vector(size);
        Self {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(1., 1., 1.),
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_xyz(pos.0 + size.x / 2., pos.1, PLATFORM_Z),
                ..default()
            },
            collider: Collider::rectangle(size.x, size.y),
            rigid_body: RigidBody::Static,
        }
    }

    pub fn with_rigid_body(mut self, rb: RigidBody) -> Self {
        self.rigid_body = rb;
        self
    }

    pub fn make_size_vector(size: f32) -> Vec2 {
        Vec2::new(size, PLATFORM_THICKNESS)
    }
}

#[derive(Bundle)]
struct MovingPlatformBundle {
    ty: MovingPlatformType,
    platform: MovingPlatform,
    shape_caster: ShapeCaster,
}

impl MovingPlatformBundle {
    pub fn slider(a: Vec3, b: Vec3, size: f32, speed: f32) -> Self {
        let size_vec = PlatformBundle::make_size_vector(size);
        Self {
            ty: MovingPlatformType::slider(a, b, speed),
            platform: MovingPlatform::default(),
            shape_caster: ShapeCaster::new(
                Collider::rectangle(size_vec.x, size_vec.y),
                Vector::ZERO,
                0.,
                Dir2::Y,
            )
            .with_ignore_origin_penetration(true)
            .with_max_time_of_impact(1.),
        }
    }
}

#[derive(Default, Resource)]
pub struct SpikeData {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<ColorMaterial>>,
}

impl SpikeData {
    pub fn ensure_initialized(
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

    pub fn mesh(&self) -> Option<Handle<Mesh>> {
        self.mesh.as_ref().map(Handle::clone_weak)
    }

    pub fn material(&self) -> Option<Handle<ColorMaterial>> {
        self.material.as_ref().map(Handle::clone_weak)
    }
}

macro_rules! level_generator {
    ($name:ident, $lower_name:ident, $func:expr) => {
        pub(super) trait $name {
            fn $lower_name(&mut self);
        }

        impl<'a> $name for LevelGenerator<'a> {
            fn $lower_name(&mut self) {
                $func(self)
            }
        }
    };
}

use level_generator;

pub struct LevelGenerator<'a> {
    commands: Commands<'a, 'a>,
    level_commands: EntityCommands<'a>,
    spike_data: ResMut<'a, SpikeData>,
}

impl<'a> LevelGenerator<'a> {
    pub fn new(
        commands: Commands<'a, 'a>,
        level_commands: EntityCommands<'a>,
        meshes: ResMut<Assets<Mesh>>,
        materials: ResMut<Assets<ColorMaterial>>,
        mut spike_data: ResMut<'a, SpikeData>,
    ) -> Self {
        spike_data.ensure_initialized(meshes, materials);
        Self {
            commands,
            level_commands,
            spike_data,
        }
    }

    pub fn setup_level(
        commands: Commands<'a, 'a>,
        level_commands: EntityCommands<'a>,
        meshes: ResMut<Assets<Mesh>>,
        materials: ResMut<Assets<ColorMaterial>>,
        spike_data: ResMut<'a, SpikeData>,
        idx: u16,
    ) {
        let mut lg = Self::new(commands, level_commands, meshes, materials, spike_data);
        lg.spawn_level_text(idx);
        match idx {
            0 => lg.level0(),
            1 => lg.level1(),
            _ => panic!("Invalid level index: {idx}"),
        }
    }

    pub fn level_count() -> u16 {
        2
    }

    fn spawn_level_text(&mut self, index: u16) {
        let id = self
            .commands
            .spawn(Text2dBundle {
                text: Text::from_section(
                    format!("Level {}", index + 1),
                    TextStyle {
                        color: GRAY.with_alpha(0.2).into(),
                        font_size: 80.,
                        ..default()
                    },
                ),
                transform: Transform::from_xyz(0., 0., LEVEL_TEXT_Z),
                ..default()
            })
            .id();
        self.level_commands.add_child(id);
    }

    fn platform(&mut self, pos: (f32, f32), size: f32) {
        let id = self.commands.spawn(PlatformBundle::new(pos, size)).id();
        self.level_commands.add_child(id);
    }

    /// speed: u/s
    fn slider_platform(&mut self, a: (f32, f32), b: (f32, f32), size: f32, speed: f32) {
        let id = self
            .commands
            .spawn((
                PlatformBundle::new(a, size).with_rigid_body(RigidBody::Kinematic),
                MovingPlatformBundle::slider(
                    Vec3::new(a.0 + size / 2., a.1, PLATFORM_Z),
                    Vec3::new(b.0 + size / 2., b.1, PLATFORM_Z),
                    size,
                    speed,
                ),
            ))
            .id();
        self.level_commands.add_child(id);
    }

    fn spike(&mut self, pos: (f32, f32)) {
        self.commands.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(self.spike_data.mesh().unwrap()),
                material: self.spike_data.material().unwrap(),
                transform: Transform::from_xyz(pos.0, pos.1, SPIKE_Z),
                visibility: Visibility::Hidden,
                ..default()
            },
            Spike,
            Collider::rectangle(SPIKE_SIZE.x, SPIKE_SIZE.y),
        ));
    }

    fn ending(&mut self, pos: (f32, f32)) {
        let id = self
            .commands
            .spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(1., 1., 1., 0.8),
                        custom_size: Some(DOOR_SIZE),
                        ..default()
                    },
                    transform: Transform::from_xyz(pos.0, pos.1 + DOOR_SIZE.y / 2., DOOR_Z),
                    ..default()
                },
                LevelEnd,
                Collider::rectangle(DOOR_SIZE.x, DOOR_SIZE.y),
            ))
            .id();
        self.level_commands.add_child(id);
    }
}
