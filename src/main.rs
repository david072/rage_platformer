use avian2d::{math::Vector, prelude::*};
use bevy::prelude::*;
use character_controller::{CharacterControllerBundle, CharacterControllerPlugin};

mod character_controller;
mod level0;

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
        .insert_resource(Gravity(Vector::NEG_Y * 1000.))
        .add_systems(Startup, setup)
        .add_systems(Update, camera_smooth_follow_player)
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
            transform: Transform {
                translation: Vec3::new(0., 30., 0.),
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

    level0::setup(commands);
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
