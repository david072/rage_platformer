use avian2d::{math::Vector, prelude::*};
use bevy::{color::palettes::css::*, ecs::system::EntityCommands, prelude::*, time::Stopwatch};
use character_controller::{CharacterControllerBundle, CharacterControllerPlugin};
use levels::{LevelEnd, LevelGenerator, MovingPlatform, MovingPlatformType, Spike, SpikeData};
use ui::{main_menu::MainMenuPlugin, pause_menu::PauseMenuPlugin};

mod character_controller;
mod levels;
mod ui;

const PLATFORM_SPEED: f32 = 0.5;
const BOTTOM_WORLD_BOUNDARY: f32 = -500.;

#[derive(Debug, Clone, PartialEq, Eq, Hash, States)]
pub enum GameState {
    MainMenu,
    LevelSelect,
    Level { index: u16, paused: bool },
}

impl GameState {
    pub fn level(index: u16) -> Self {
        Self::Level {
            index,
            paused: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct InLevel;

impl ComputedStates for InLevel {
    type SourceStates = GameState;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            GameState::Level { .. } => Some(Self),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsPaused {
    Paused,
    Unpaused,
}

impl ComputedStates for IsPaused {
    type SourceStates = GameState;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            GameState::Level { paused: true, .. } => Some(Self::Paused),
            GameState::Level { paused: false, .. } => Some(Self::Unpaused),
            _ => None,
        }
    }
}

#[derive(Event)]
struct LevelCompleteEvent;

#[derive(Event)]
enum LevelRestartEvent {
    KeepSpikes,
    /// Performs a full reset and spawns the given level index.
    /// We need to add the level id since the state changes aren't committed in the same frame,
    /// meaning setup_level_content doesn't get the correct index directly.
    FullReset(u16),
}

#[derive(Default, Resource)]
struct LevelStopwatch(Stopwatch);

#[derive(Default, Resource)]
struct DeathCounter(usize);

#[derive(Event)]
struct DeathEvent;

#[derive(Component)]
struct LevelRoot;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct LevelText;

#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct DeathsText;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // 1 meter = 20 pixels
            PhysicsPlugins::default().with_length_unit(20.),
        ))
        .add_plugins(CharacterControllerPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(PauseMenuPlugin)
        .add_event::<LevelCompleteEvent>()
        .add_event::<LevelRestartEvent>()
        .add_event::<DeathEvent>()
        .insert_resource(Gravity(Vector::NEG_Y * 1000.))
        .insert_resource(SpikeData::default())
        .insert_resource(DeathCounter::default())
        .init_resource::<LevelStopwatch>()
        .add_computed_state::<InLevel>()
        .add_computed_state::<IsPaused>()
        .insert_state(GameState::level(0))
        .add_systems(Startup, setup)
        .add_systems(OnEnter(InLevel), setup_level)
        .add_systems(OnEnter(IsPaused::Paused), begin_pause)
        .add_systems(OnExit(IsPaused::Paused), end_pause)
        .add_systems(OnExit(InLevel), (cleanup_level, cleanup_level_content))
        .add_systems(
            Update,
            (
                camera_smooth_follow_player,
                moving_platform_system,
                (
                    level_complete_condition,
                    on_level_completed,
                    death_condition,
                    setup_level_content,
                )
                    .chain(),
            )
                .run_if(in_state(IsPaused::Unpaused)),
        )
        .add_systems(
            PostUpdate,
            (update_death_counter, update_hud)
                .chain()
                .run_if(in_state(IsPaused::Unpaused)),
        )
        .add_systems(Update, pause_system.run_if(in_state(InLevel)))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_level(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    mut level_changed_writer: EventWriter<LevelRestartEvent>,
) {
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
        CharacterControllerBundle::new(Collider::capsule(10., 20.)),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        ColliderDensity(2.),
        ExternalForce::new(Vector::ZERO).with_persistence(false),
    ));

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    margin: UiRect::all(Val::Px(20.)),
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            },
            Hud,
        ))
        .with_children(|parent| {
            fn text<'a>(
                parent: &'a mut ChildBuilder,
                text: impl Into<String>,
                font_size: f32,
            ) -> EntityCommands<'a> {
                parent.spawn(TextBundle::from_section(
                    text,
                    TextStyle {
                        font_size,
                        color: WHITE.into(),
                        ..default()
                    },
                ))
            }

            text(parent, "Level 1", 50.).insert(LevelText);

            parent.spawn(NodeBundle {
                style: Style {
                    margin: UiRect::vertical(Val::Px(10.)),
                    width: Val::Px(100.),
                    height: Val::Px(4.),
                    ..default()
                },
                background_color: LIGHT_SLATE_GRAY.into(),
                ..default()
            });

            text(parent, "Time: 12.1s", 25.).insert(TimeText);
            text(parent, "Deaths: 0", 25.).insert(DeathsText);
        });

    commands.insert_resource(LevelStopwatch::default());

    let GameState::Level { index, .. } = **game_state else {
        return;
    };
    level_changed_writer.send(LevelRestartEvent::FullReset(index));
}

fn cleanup_level(
    mut commands: Commands,
    player: Query<Entity, With<Player>>,
    hud: Query<Entity, With<Hud>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    commands.entity(player).despawn_recursive();

    for entity in &hud {
        commands.entity(entity).despawn_recursive();
    }

    commands.remove_resource::<LevelStopwatch>();
}

fn setup_level_content(
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
    }

    let mut player = player.single_mut();
    player.translation = Vec3::ZERO;

    let mut level_idx: Option<u16> = None;
    if let LevelRestartEvent::FullReset(idx) = level_restart_event {
        level_idx = Some(*idx);
        for spike in &spikes {
            commands.entity(spike).despawn_recursive();
        }
    }

    // spawn level entities
    let Some(level_idx) = level_idx.or_else(|| {
        if let GameState::Level { index, .. } = **game_state {
            Some(index)
        } else {
            None
        }
    }) else {
        return;
    };

    let level_root = commands.spawn((
        LevelRoot,
        TransformBundle::default(),
        VisibilityBundle::default(),
    ));
    LevelGenerator::setup_level(
        commands2, level_root, meshes, materials, spike_data, level_idx,
    );
}

fn cleanup_level_content(mut commands: Commands, level_root: Query<Entity, With<LevelRoot>>) {
    if let Ok(level_root) = level_root.get_single() {
        commands.entity(level_root).despawn_recursive();
    }
}

fn camera_smooth_follow_player(
    mut cameras: Query<&mut Transform, With<Camera2d>>,
    player: Query<&Transform, (With<Player>, Without<Camera2d>)>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };

    for mut camera in &mut cameras {
        camera.translation = camera.translation.lerp(player.translation, 0.1);
    }
}

fn level_complete_condition(
    player: Query<Entity, With<Player>>,
    level_end: Query<&CollidingEntities, With<LevelEnd>>,
    mut level_complete_writer: EventWriter<LevelCompleteEvent>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    for end_colliding_entities in &level_end {
        for entity in end_colliding_entities.iter() {
            if *entity != player {
                continue;
            }

            level_complete_writer.send(LevelCompleteEvent);
            return;
        }
    }
}

fn on_level_completed(
    mut level_stopwatch: ResMut<LevelStopwatch>,
    mut death_counter: ResMut<DeathCounter>,
    mut level_complete_reader: EventReader<LevelCompleteEvent>,
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut level_restart_writer: EventWriter<LevelRestartEvent>,
) {
    if level_complete_reader.read().count() == 0 {
        return;
    }

    let GameState::Level { index, .. } = **game_state else {
        return;
    };
    next_state.set(GameState::level(index + 1));
    level_restart_writer.send(LevelRestartEvent::FullReset(index + 1));
    level_stopwatch.0.reset();
    death_counter.0 = 0;
}

fn death_condition(
    player: Query<(Entity, &Transform), With<Player>>,
    mut spikes: Query<(&CollidingEntities, &mut Visibility), With<Spike>>,
    mut death_event_writer: EventWriter<DeathEvent>,
    mut level_restart_writer: EventWriter<LevelRestartEvent>,
) {
    let Ok((player, player_transform)) = player.get_single() else {
        return;
    };

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

fn update_death_counter(
    mut death_counter: ResMut<DeathCounter>,
    mut death_event_reader: EventReader<DeathEvent>,
) {
    for _ in death_event_reader.read() {
        death_counter.0 += 1;
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
        if !top_hits.is_empty() {
            platform.active = true;
        }

        let movement_sign = if platform.moving_backward { -1. } else { 1. };
        if platform.active {
            platform.t += PLATFORM_SPEED * time.delta_seconds() * movement_sign;
            platform.t = platform.t.clamp(0., 1.);

            if platform.t >= 1.0 {
                platform.moving_backward = true;
            } else if platform.t <= 0.0 {
                platform.moving_backward = false;
            }
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

fn pause_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !keyboard_input.just_pressed(KeyCode::Escape) {
        return;
    }

    let GameState::Level { index, paused } = **game_state else {
        return;
    };
    let now_paused = !paused;
    next_state.set(GameState::Level {
        index,
        paused: now_paused,
    });
}

fn begin_pause(mut physics_time: ResMut<Time<Physics>>) {
    physics_time.pause();
}

fn end_pause(mut physics_time: ResMut<Time<Physics>>) {
    physics_time.unpause();
}

fn update_hud(
    time: Res<Time<Physics>>,
    mut level_stopwatch: ResMut<LevelStopwatch>,
    game_state: Res<State<GameState>>,
    deaths: Res<DeathCounter>,
    mut texts: Query<(&mut Text, Has<LevelText>, Has<TimeText>, Has<DeathsText>)>,
) {
    let GameState::Level {
        index: level_idx, ..
    } = **game_state
    else {
        return;
    };

    level_stopwatch.0.tick(time.delta());

    for (mut text, is_level_text, is_time_text, is_deaths_text) in &mut texts {
        text.sections[0].value = if is_level_text {
            format!("Level {}", level_idx + 1)
        } else if is_time_text {
            format!("Time: {:.1}s", level_stopwatch.0.elapsed_secs())
        } else if is_deaths_text {
            format!("Deaths: {}", deaths.0)
        } else {
            continue;
        };
    }
}
