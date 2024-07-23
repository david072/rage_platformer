use avian2d::{math::Vector, prelude::*};
use bevy::prelude::*;
use character_controller::{CharacterControllerBundle, CharacterControllerPlugin};
use levels::{LevelEnd, LevelGenerator, MovingPlatform, MovingPlatformType, Spike, SpikeData};

mod character_controller;
mod levels;

const PLATFORM_SPEED: f32 = 0.5;
const BOTTOM_WORLD_BOUNDARY: f32 = -500.;

#[derive(Debug, Clone, PartialEq, Eq, Hash, States)]
enum GameState {
    Level(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct InLevel;

impl ComputedStates for InLevel {
    type SourceStates = GameState;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            GameState::Level(_) => Some(Self),
        }
    }
}

#[derive(Event)]
enum LevelRestartEvent {
    KeepSpikes,
    FullReset,
}

#[derive(Event)]
struct DeathEvent;

#[derive(Component)]
struct LevelRoot;

#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // 1 meter = 20 pixels
            PhysicsPlugins::default().with_length_unit(20.),
        ))
        .add_plugins(CharacterControllerPlugin)
        .add_event::<LevelRestartEvent>()
        .add_event::<DeathEvent>()
        .insert_resource(Gravity(Vector::NEG_Y * 1000.))
        .insert_resource(SpikeData::default())
        .add_computed_state::<InLevel>()
        .insert_state(GameState::Level(0))
        .add_systems(OnEnter(InLevel), setup)
        .add_systems(
            Update,
            (
                (setup_level, level_complete_condition, death_condition).chain(),
                camera_smooth_follow_player,
                moving_platform_system,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, mut level_changed_writer: EventWriter<LevelRestartEvent>) {
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
        ExternalForce::new(Vector::ZERO).with_persistence(false),
    ));

    level_changed_writer.send(LevelRestartEvent::FullReset);
}

fn setup_level(
    mut level_restart_reader: EventReader<LevelRestartEvent>,
    level_root: Query<Entity, With<LevelRoot>>,
    spikes: Query<Entity, With<Spike>>,
    mut player: Query<&mut Transform, With<Player>>,
    // The EntityCommands that we get from Commands::spawn() reborrows the Commands, which means
    // we cannot borrow it again when passing it to setup_level. Therefore, we just ask Bevy to
    // give us another 'static Commands lol...
    mut commands: Commands,
    commands2: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    spike_data: ResMut<SpikeData>,
    game_state: Res<State<GameState>>,
) {
    // reset level
    let Some(level_restart_event) = level_restart_reader.read().next() else {
        return;
    };

    if let Ok(level_root) = level_root.get_single() {
        commands.entity(level_root).despawn_recursive();

        let mut player = player.single_mut();
        player.translation = Vec3::ZERO;
    }

    if matches!(level_restart_event, LevelRestartEvent::FullReset) {
        for spike in &spikes {
            commands.entity(spike).despawn_recursive();
        }
    }

    // spawn level entities
    let GameState::Level(idx) = **game_state;

    let level_root = commands.spawn((
        LevelRoot,
        TransformBundle::default(),
        VisibilityBundle::default(),
    ));
    LevelGenerator::setup_level(commands2, level_root, meshes, materials, spike_data, idx);
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

fn level_complete_condition(
    mut commands: Commands,
    player: Query<Entity, With<Player>>,
    level_end: Query<(Entity, &CollidingEntities), With<LevelEnd>>,
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut level_restart_writer: EventWriter<LevelRestartEvent>,
) {
    let player = player.single();
    for (end_entity, end_colliding_entities) in &level_end {
        for entity in end_colliding_entities.iter() {
            if *entity != player {
                continue;
            }

            let GameState::Level(idx) = **game_state;
            next_state.set(GameState::Level(idx + 1));
            level_restart_writer.send(LevelRestartEvent::FullReset);
            commands.entity(end_entity).remove::<Collider>();
            return;
        }
    }
}

fn death_condition(
    player: Query<(Entity, &Transform), With<Player>>,
    mut spikes: Query<(&CollidingEntities, &mut Visibility), With<Spike>>,
    mut death_event_writer: EventWriter<DeathEvent>,
    mut level_restart_writer: EventWriter<LevelRestartEvent>,
) {
    let (player, player_transform) = player.single();

    for (colliding_entities, mut visibility) in &mut spikes {
        if !colliding_entities.contains(&player) {
            continue;
        }

        *visibility = Visibility::default();
        death_event_writer.send(DeathEvent);
        level_restart_writer.send(LevelRestartEvent::KeepSpikes);
        return;
    }

    if player_transform.translation.y <= BOTTOM_WORLD_BOUNDARY {
        death_event_writer.send(DeathEvent);
        level_restart_writer.send(LevelRestartEvent::KeepSpikes);
    }
}

fn moving_platform_system(
    time: Res<Time>,
    mut platforms: Query<(
        &mut Transform,
        &MovingPlatformType,
        &mut MovingPlatform,
        &ShapeHits,
    )>,
    mut rigid_bodies: Query<(&RigidBody, &mut Transform), Without<MovingPlatform>>,
) {
    for (mut transform, ty, mut platform, top_hits) in &mut platforms {
        let movement_sign = if platform.moving_backward { -1. } else { 1. };
        platform.t += PLATFORM_SPEED * time.delta_seconds() * movement_sign;
        platform.t = platform.t.clamp(0., 1.);

        if platform.t >= 1.0 {
            platform.moving_backward = true;
        } else if platform.t <= 0.0 {
            platform.moving_backward = false;
        }

        match ty {
            MovingPlatformType::Slider(a, b) => {
                transform.translation = a.lerp(*b, platform.t);

                // units per second
                // It takes the platform 1 / PLATFORM_SPEED to go from a to b, i.e. the distance of |a.x - b.x|. This means
                // platform_speed = (a.x - b.x).abs() / (1. / PLATFORM_SPEED)
                // Adding the movement sign and simplifying leads to the following:
                let platform_speed = (a.x - b.x).abs() * PLATFORM_SPEED * movement_sign;

                // FIXME: This moves the RigidBody into other colliders and it causes weird stuff :( pls fix
                for ShapeHitData { entity, .. } in top_hits.iter() {
                    let Ok((rb, mut transform)) = rigid_bodies.get_mut(*entity) else {
                        continue;
                    };
                    if !matches!(rb, RigidBody::Dynamic) {
                        continue;
                    }
                    transform.translation.x += platform_speed * time.delta_seconds();
                }
            }
        }
    }
}
