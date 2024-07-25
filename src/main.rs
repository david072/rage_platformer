use avian2d::{math::Vector, prelude::*};
use bevy::{
    audio::{PlaybackMode, Volume},
    color::palettes::css::*,
    ecs::system::EntityCommands,
    prelude::*,
    text::{Text2dBounds, TextLayoutInfo},
    time::Stopwatch,
};
use character_controller::{CharacterControllerBundle, CharacterControllerPlugin};
use levels::{
    persistent_anchor_system, persistent_collider_constructor_system, Checkpoint, CheckpointData,
    LevelEnd, LevelGenerator, MovingPlatform, MovingPlatformType, PersistentAnchor,
    PersistentColliderConstructor, Spike, SpikeData,
};
use ui::{main_menu::MainMenuPlugin, pause_menu::PauseMenuPlugin, UiPlugin};

mod character_controller;
mod levels;
mod ui;

const PLAYER_SIZE: Vec2 = Vec2::new(20., 40.);
const BOTTOM_WORLD_BOUNDARY: f32 = -500.;
const BACKGROUND_AUDIO: &str = "background.ogg";
const CHECKPOINT_ACTIVATE_SOUND_EFFECT: &str = "checkpoint_activate.ogg";
const DEATH_SOUND_EFFECT: &str = "player_death.ogg";
const LEVEL_COMPLETE_SOUND_EFFECT: &str = "level_complete.ogg";

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
    RestoreLastSave,
    /// Performs a full reset and spawns the given level index.
    /// We need to add the level id since the state changes aren't committed in the same frame,
    /// meaning setup_level_content doesn't get the correct index directly.
    FullReset(u16),
}

#[derive(Default, Resource)]
struct LevelStopwatch(Stopwatch);

#[derive(Default, Resource)]
struct DeathCounter(usize);

#[derive(Resource)]
struct SaveData {
    scene: Handle<DynamicScene>,
    position: Vec2,
}

#[derive(Event)]
struct DeathEvent;

#[derive(Event)]
struct CheckpointSaveEvent {
    position: Vec2,
}

#[derive(Event)]
struct RemoveSaveEvent;

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

#[derive(Component)]
struct BackgroundAudio;

fn main() {
    App::new()
        .register_type::<PersistentColliderConstructor>()
        .register_type::<MovingPlatformType>()
        .register_type::<MovingPlatform>()
        .register_type::<LevelEnd>()
        .register_type::<Text>()
        .register_type::<TextStyle>()
        .register_type::<PersistentAnchor>()
        .register_type::<Text2dBounds>()
        .register_type::<TextLayoutInfo>()
        .add_plugins((
            DefaultPlugins,
            // 1 meter = 20 pixels
            PhysicsPlugins::default().with_length_unit(20.),
            CharacterControllerPlugin,
            UiPlugin,
            MainMenuPlugin,
            PauseMenuPlugin,
        ))
        .add_event::<LevelCompleteEvent>()
        .add_event::<LevelRestartEvent>()
        .add_event::<DeathEvent>()
        .add_event::<CheckpointSaveEvent>()
        .add_event::<RemoveSaveEvent>()
        .insert_resource(Gravity(Vector::NEG_Y * 1000.))
        .insert_resource(SpikeData::default())
        .insert_resource(CheckpointData::default())
        .insert_resource(DeathCounter::default())
        .init_resource::<LevelStopwatch>()
        .add_computed_state::<InLevel>()
        .add_computed_state::<IsPaused>()
        .insert_state(GameState::MainMenu)
        .add_systems(Startup, setup)
        .add_systems(OnEnter(InLevel), setup_level)
        .add_systems(OnEnter(IsPaused::Paused), begin_pause)
        .add_systems(OnExit(IsPaused::Paused), end_pause)
        .add_systems(
            OnExit(InLevel),
            (cleanup_level, cleanup_level_content, remove_save),
        )
        .add_systems(
            Update,
            (
                camera_smooth_follow_player,
                moving_platform_system,
                (
                    checkpoint_system,
                    create_save.pipe(store_save),
                    checkpoint_load,
                )
                    .chain(),
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
            (
                play_checkpoint_activate_sound_effect,
                play_death_sound_effect,
                (update_death_counter, update_hud).chain(),
                persistent_collider_constructor_system,
                persistent_anchor_system,
            )
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
    asset_server: Res<AssetServer>,
    game_state: Res<State<GameState>>,
    mut level_changed_writer: EventWriter<LevelRestartEvent>,
) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(1., 0.7, 0.),
                custom_size: Some(PLAYER_SIZE),
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

    commands.spawn((
        AudioBundle {
            source: asset_server.load(BACKGROUND_AUDIO),
            settings: PlaybackSettings::LOOP.with_volume(Volume::new(0.2)),
        },
        BackgroundAudio,
    ));

    let GameState::Level { index, .. } = **game_state else {
        return;
    };
    level_changed_writer.send(LevelRestartEvent::FullReset(index));
}

fn cleanup_level(
    mut commands: Commands,
    player: Query<Entity, With<Player>>,
    hud: Query<Entity, With<Hud>>,
    background_audio: Query<Entity, With<BackgroundAudio>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    commands.entity(player).despawn_recursive();

    for entity in hud.iter().chain(background_audio.iter()) {
        commands.entity(entity).despawn_recursive();
    }

    commands.remove_resource::<LevelStopwatch>();
}

fn remove_save(
    mut commands: Commands,
    save_data: Option<Res<SaveData>>,
    mut dynamic_scenes: ResMut<Assets<DynamicScene>>,
) {
    if let Some(save_data) = save_data {
        dynamic_scenes.remove(&save_data.scene);
        commands.remove_resource::<SaveData>();
    }
}

fn setup_level_content(
    mut level_restart_reader: EventReader<LevelRestartEvent>,
    level_root: Query<Entity, With<LevelRoot>>,
    spikes: Query<Entity, With<Spike>>,
    checkpoints: Query<Entity, With<Checkpoint>>,
    mut player: Query<(&mut Transform, Option<&mut LinearVelocity>), With<Player>>,
    // The EntityCommands that we get from Commands::spawn() reborrows the Commands, which means
    // we cannot borrow it again when passing it to setup_level. Therefore, we just ask Bevy to
    // give us another 'static Commands lol...
    mut commands: Commands,
    commands2: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    spike_data: ResMut<SpikeData>,
    checkpoint_data: ResMut<CheckpointData>,
    game_state: Res<State<GameState>>,
    save_data: Option<Res<SaveData>>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    // reset level
    let Some(level_restart_event) = level_restart_reader.read().next() else {
        return;
    };

    if let Ok(level_root) = level_root.get_single() {
        commands.entity(level_root).despawn_recursive();
    }

    let (mut player_transform, player_velocity) = player.single_mut();
    if let Some(mut vel) = player_velocity {
        vel.0 = Vector::ZERO;
    }

    match level_restart_event {
        LevelRestartEvent::RestoreLastSave => {
            let level_root = commands.spawn((
                LevelRoot,
                TransformBundle::default(),
                VisibilityBundle::default(),
            ));

            if let Some(save_data) = save_data {
                player_transform.translation = save_data.position.extend(0.);
                scene_spawner.spawn_dynamic_as_child(save_data.scene.clone_weak(), level_root.id());
            } else {
                player_transform.translation = Vec3::ZERO;
                let GameState::Level { index, .. } = **game_state else {
                    return;
                };

                LevelGenerator::setup_level_without_permanent_entities(
                    commands2,
                    level_root,
                    &mut meshes,
                    &mut materials,
                    spike_data,
                    checkpoint_data,
                    index,
                );
            }
        }
        LevelRestartEvent::FullReset(index) => {
            player_transform.translation = Vec3::ZERO;
            for entity in spikes.iter().chain(checkpoints.iter()) {
                commands.entity(entity).despawn_recursive();
            }

            let level_root = commands.spawn((
                LevelRoot,
                TransformBundle::default(),
                VisibilityBundle::default(),
            ));
            LevelGenerator::setup_level(
                commands2,
                level_root,
                &mut meshes,
                &mut materials,
                spike_data,
                checkpoint_data,
                *index,
            );
        }
    }
}

fn cleanup_level_content(
    mut commands: Commands,
    level_root: Query<Entity, With<LevelRoot>>,
    spikes: Query<Entity, With<Spike>>,
    checkpoints: Query<Entity, With<Checkpoint>>,
) {
    if let Ok(level_root) = level_root.get_single() {
        commands.entity(level_root).despawn_recursive();
    }

    for entity in spikes.iter().chain(checkpoints.iter()) {
        commands.entity(entity).despawn_recursive();
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
    mut commands: Commands,
    save_data: Option<Res<SaveData>>,
    dynamic_scenes: ResMut<Assets<DynamicScene>>,
    asset_server: Res<AssetServer>,
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

    commands.spawn(AudioBundle {
        source: asset_server.load(LEVEL_COMPLETE_SOUND_EFFECT),
        settings: PlaybackSettings::DESPAWN.with_volume(Volume::new(0.5)),
    });

    remove_save(commands, save_data, dynamic_scenes);
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
        level_restart_writer.send(LevelRestartEvent::RestoreLastSave);
        return;
    }

    if player_transform.translation.y <= BOTTOM_WORLD_BOUNDARY {
        death_event_writer.send(DeathEvent);
        level_restart_writer.send(LevelRestartEvent::RestoreLastSave);
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

fn play_death_sound_effect(
    mut commands: Commands,
    mut death_event_reader: EventReader<DeathEvent>,
    asset_server: Res<AssetServer>,
) {
    for _ in death_event_reader.read() {
        commands.spawn(AudioBundle {
            source: asset_server.load(DEATH_SOUND_EFFECT),
            settings: PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::new(0.3),
                ..default()
            },
        });
    }
}

fn moving_platform_system(
    time: Res<Time>,
    mut platforms: Query<(
        &mut Transform,
        &MovingPlatformType,
        &mut MovingPlatform,
        &CollidingEntities,
    )>,
    mut rigid_bodies: Query<(&RigidBody, &mut Transform), Without<MovingPlatform>>,
) {
    for (mut transform, ty, mut platform, colliding_entities) in &mut platforms {
        if !colliding_entities.is_empty() {
            platform.active = true;
        }

        if !platform.active {
            continue;
        }

        let movement_sign = if platform.moving_backward { -1. } else { 1. };

        match ty {
            MovingPlatformType::Slider {
                a,
                b,
                speed,
                delta_t_per_second,
            } => {
                platform.t += delta_t_per_second * time.delta_seconds() * movement_sign;
                platform.t = platform.t.clamp(0., 1.);

                transform.translation = a.lerp(*b, platform.t);

                // FIXME: This moves the RigidBody into other colliders and it causes weird stuff :( pls fix
                for entity in colliding_entities.iter() {
                    let Ok((rb, mut transform)) = rigid_bodies.get_mut(*entity) else {
                        continue;
                    };
                    if !matches!(rb, RigidBody::Dynamic) {
                        continue;
                    }
                    transform.translation.x += speed * time.delta_seconds() * movement_sign;
                }
            }
        }

        if platform.t >= 1.0 {
            platform.moving_backward = true;
        } else if platform.t <= 0.0 {
            platform.moving_backward = false;
        }
    }
}

fn checkpoint_system(
    player: Query<Entity, With<Player>>,
    mut checkpoints: Query<(
        Entity,
        &Transform,
        &CollidingEntities,
        &mut Checkpoint,
        &mut Handle<ColorMaterial>,
    )>,
    checkpoint_data: ResMut<CheckpointData>,
    mut save_event_writer: EventWriter<CheckpointSaveEvent>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let mut active_checkpoint: Option<Entity> = None;
    for (entity, transform, colliding_entities, mut checkpoint, mut material) in &mut checkpoints {
        let is_active = checkpoint.active;
        if checkpoint.active {
            checkpoint.active = false;
            *material = checkpoint_data.default_material().unwrap();

            if active_checkpoint.is_none() {
                active_checkpoint = Some(entity);
            }
        }

        if colliding_entities.iter().any(|e| *e == player) {
            active_checkpoint = Some(entity);
            if !is_active {
                save_event_writer.send(CheckpointSaveEvent {
                    position: (transform.translation).truncate() + Vec2::new(0., PLAYER_SIZE.y),
                });
            }
        }
    }

    if let Some(cp) = active_checkpoint {
        let (.., mut checkpoint, mut material) = checkpoints.get_mut(cp).unwrap();
        checkpoint.active = true;
        *material = checkpoint_data.active_material().unwrap();
    }
}

fn play_checkpoint_activate_sound_effect(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut save_event_reader: EventReader<CheckpointSaveEvent>,
) {
    for _ in save_event_reader.read() {
        commands.spawn(AudioBundle {
            source: asset_server.load(CHECKPOINT_ACTIVATE_SOUND_EFFECT),
            settings: PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::new(0.3),
                ..default()
            },
        });
    }
}

fn create_save(
    mut save_event_reader: EventReader<CheckpointSaveEvent>,
    level_root: Query<&Children, With<LevelRoot>>,
    world: &World,
) -> Option<(Vec2, DynamicScene)> {
    let Some(CheckpointSaveEvent { position }) = save_event_reader.read().next() else {
        return None;
    };
    let Ok(level_root_children) = level_root.get_single() else {
        return None;
    };

    let dynamic_scene = DynamicSceneBuilder::from_world(world)
        .deny::<Parent>()
        .extract_entities(level_root_children.iter().map(|e| *e))
        .build();

    Some((*position, dynamic_scene))
}

fn store_save(
    In(created_save): In<Option<(Vec2, DynamicScene)>>,
    mut commands: Commands,
    mut dynamic_scenes: ResMut<Assets<DynamicScene>>,
    save_data: Option<ResMut<SaveData>>,
) {
    let Some((position, dynamic_scene)) = created_save else {
        return;
    };

    if let Some(mut save_data) = save_data {
        dynamic_scenes.remove(&save_data.scene);
        save_data.scene = dynamic_scenes.add(dynamic_scene);
        save_data.position = position;
    } else {
        commands.insert_resource(SaveData {
            scene: dynamic_scenes.add(dynamic_scene),
            position,
        });
    }
}

fn checkpoint_load(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut level_restart_writer: EventWriter<LevelRestartEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        level_restart_writer.send(LevelRestartEvent::RestoreLastSave);
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

fn begin_pause(
    mut physics_time: ResMut<Time<Physics>>,
    mut background_audio: Query<&mut AudioSink, With<BackgroundAudio>>,
) {
    physics_time.pause();
    for sink in &mut background_audio {
        sink.pause();
    }
}

fn end_pause(
    mut physics_time: ResMut<Time<Physics>>,
    mut background_audio: Query<&mut AudioSink, With<BackgroundAudio>>,
) {
    physics_time.unpause();
    for sink in &mut background_audio {
        sink.play();
    }
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
