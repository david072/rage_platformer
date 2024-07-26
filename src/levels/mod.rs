use avian2d::prelude::*;
use bevy::{
    color::palettes::css::*,
    ecs::system::EntityCommands,
    prelude::*,
    sprite::{Anchor, MaterialMesh2dBundle, Mesh2dHandle},
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

#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct LevelEnd;

#[derive(Default, Component)]
pub struct Spike {
    pub group: Option<usize>,
}

#[derive(Default, Component)]
pub struct Checkpoint {
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Reflect, Component)]
#[reflect(Debug, Component, PartialEq)]
pub struct PersistentColliderConstructor(ColliderConstructor);

impl From<ColliderConstructor> for PersistentColliderConstructor {
    fn from(value: ColliderConstructor) -> Self {
        Self(value)
    }
}

pub fn persistent_collider_constructor_system(
    mut commands: Commands,
    entities: Query<(
        Entity,
        &PersistentColliderConstructor,
        Has<Collider>,
        Has<ColliderConstructor>,
    )>,
) {
    for (entity, persistent_collider_constructor, has_collider, has_collider_constructor) in
        &entities
    {
        if has_collider || has_collider_constructor {
            continue;
        }

        commands
            .entity(entity)
            .insert(persistent_collider_constructor.0.clone());
    }
}

#[derive(Clone, Debug, PartialEq, Reflect, Component)]
#[reflect(Debug, Component, PartialEq)]
pub struct PersistentAnchor(Anchor);

impl From<Anchor> for PersistentAnchor {
    fn from(value: Anchor) -> Self {
        Self(value)
    }
}

pub fn persistent_anchor_system(
    mut commands: Commands,
    entities: Query<(Entity, &PersistentAnchor, Has<Anchor>)>,
) {
    for (entity, persistent_anchor, has_anchor) in &entities {
        if has_anchor {
            continue;
        }

        commands.entity(entity).insert(persistent_anchor.0.clone());
    }
}

#[derive(Reflect, Component)]
#[reflect(Component)]
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

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct MovingPlatform {
    pub active: bool,
    pub t: f32,
    pub moving_backward: bool,
}

#[derive(Bundle)]
struct PlatformBundle {
    sprite: SpriteBundle,
    collider_constructor: PersistentColliderConstructor,
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
            collider_constructor: ColliderConstructor::Rectangle {
                x_length: size.x,
                y_length: size.y,
            }
            .into(),
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
}

impl MovingPlatformBundle {
    pub fn slider(a: Vec3, b: Vec3, speed: f32) -> Self {
        Self {
            ty: MovingPlatformType::slider(a, b, speed),
            platform: MovingPlatform::default(),
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
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
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

#[derive(Default, Resource)]
pub struct CheckpointData {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<ColorMaterial>>,
    active_material: Option<Handle<ColorMaterial>>,
}

impl CheckpointData {
    pub fn ensure_initialized(
        &mut self,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
    ) {
        if self.mesh.is_none() {
            self.mesh = Some(meshes.add(Triangle2d::new(
                Vec2::new(-20., 40.),
                Vec2::new(20., 40.),
                Vec2::ZERO,
            )));
        }
        if self.material.is_none() {
            self.material = Some(materials.add(Color::from(GRAY)));
        }
        if self.active_material.is_none() {
            self.active_material = Some(materials.add(Color::from(LIGHT_GREEN)));
        }
    }

    pub fn mesh(&self) -> Option<Handle<Mesh>> {
        self.mesh.as_ref().map(Handle::clone_weak)
    }

    pub fn default_material(&self) -> Option<Handle<ColorMaterial>> {
        self.material.as_ref().map(Handle::clone_weak)
    }

    pub fn active_material(&self) -> Option<Handle<ColorMaterial>> {
        self.active_material.as_ref().map(Handle::clone_weak)
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
    checkpoint_data: ResMut<'a, CheckpointData>,
    enable_permanent_entities: bool,
    current_spike_group: usize,
}

impl<'a> LevelGenerator<'a> {
    pub fn new(
        commands: Commands<'a, 'a>,
        level_commands: EntityCommands<'a>,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
        mut spike_data: ResMut<'a, SpikeData>,
        mut checkpoint_data: ResMut<'a, CheckpointData>,
    ) -> Self {
        spike_data.ensure_initialized(meshes, materials);
        checkpoint_data.ensure_initialized(meshes, materials);
        Self {
            commands,
            level_commands,
            spike_data,
            checkpoint_data,
            enable_permanent_entities: true,
            current_spike_group: 0,
        }
    }

    pub fn setup_level(
        commands: Commands<'a, 'a>,
        level_commands: EntityCommands<'a>,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
        spike_data: ResMut<'a, SpikeData>,
        checkpoint_data: ResMut<'a, CheckpointData>,
        idx: u16,
    ) {
        let mut lg = Self::new(
            commands,
            level_commands,
            meshes,
            materials,
            spike_data,
            checkpoint_data,
        );
        lg.spawn_level_text(idx);
        match idx {
            0 => lg.level0(),
            1 => lg.level1(),
            _ => panic!("Invalid level index: {idx}"),
        }
    }

    pub fn setup_level_without_permanent_entities(
        commands: Commands<'a, 'a>,
        level_commands: EntityCommands<'a>,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
        spike_data: ResMut<'a, SpikeData>,
        checkpoint_data: ResMut<'a, CheckpointData>,
        idx: u16,
    ) {
        let mut lg = Self::new(
            commands,
            level_commands,
            meshes,
            materials,
            spike_data,
            checkpoint_data,
        );
        lg.spawn_level_text(idx);
        lg.set_enable_permanent_entities(false);
        match idx {
            0 => lg.level0(),
            1 => lg.level1(),
            _ => panic!("Invalid level index: {idx}"),
        }
    }

    pub fn level_count() -> u16 {
        2
    }

    fn set_enable_permanent_entities(&mut self, enable: bool) {
        self.enable_permanent_entities = enable;
    }

    fn spawn_level_text(&mut self, index: u16) {
        let bundle = Text2dBundle {
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
        };

        let id = self
            .commands
            .spawn((PersistentAnchor(bundle.text_anchor.clone()), bundle))
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
                    speed,
                ),
            ))
            .id();
        self.level_commands.add_child(id);
    }

    fn spike_base(&mut self, pos: (f32, f32)) -> EntityCommands {
        self.commands.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(self.spike_data.mesh().unwrap()),
                material: self.spike_data.material().unwrap(),
                transform: Transform::from_xyz(pos.0, pos.1, SPIKE_Z),
                visibility: Visibility::Hidden,
                ..default()
            },
            Collider::rectangle(SPIKE_SIZE.x, SPIKE_SIZE.y),
        ))
    }

    fn spike(&mut self, pos: (f32, f32)) {
        if !self.enable_permanent_entities {
            return;
        }
        self.spike_base(pos).insert(Spike::default());
    }

    fn spike_group(&mut self, start_x: f32, end_x: f32, y: f32) {
        if !self.enable_permanent_entities {
            return;
        }

        let mut x = ((end_x - start_x) % SPIKE_SIZE.x) / 2. + start_x;
        let group = self.current_spike_group;
        while x <= end_x {
            self.spike_base((x, y)).insert(Spike { group: Some(group) });
            x += SPIKE_SIZE.x;
        }

        self.current_spike_group += 1;
    }

    fn vertical_spike_group(&mut self, x: f32, start_y: f32, end_y: f32) {
        if !self.enable_permanent_entities {
            return;
        }

        let mut y = ((end_y - start_y) % SPIKE_SIZE.y) / 2. + start_y;
        let group = self.current_spike_group;
        while y <= end_y {
            self.spike_base((x, y)).insert(Spike { group: Some(group) });
            y += SPIKE_SIZE.y;
        }

        self.current_spike_group += 1;
    }

    fn checkpoint(&mut self, pos: (f32, f32)) {
        if !self.enable_permanent_entities {
            return;
        }
        self.commands.spawn((
            MaterialMesh2dBundle {
                mesh: Mesh2dHandle(self.checkpoint_data.mesh().unwrap()),
                material: self.checkpoint_data.default_material().unwrap(),
                transform: Transform::from_xyz(pos.0, pos.1, -10.),
                ..default()
            },
            Collider::triangle(Vec2::new(-20., 40.), Vec2::new(20., 40.), Vec2::ZERO),
            Checkpoint::default(),
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
                PersistentColliderConstructor(ColliderConstructor::Rectangle {
                    x_length: DOOR_SIZE.x,
                    y_length: DOOR_SIZE.y,
                }),
            ))
            .id();
        self.level_commands.add_child(id);
    }
}
