use avian2d::{math::Vector, prelude::*};
use bevy::prelude::*;
use character_controller::{CharacterControllerBundle, CharacterControllerPlugin};

mod character_controller;

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

    // commands.spawn((
    //     SpriteBundle {
    //         sprite: Sprite {
    //             color: Color::srgb(1., 1., 1.),
    //             custom_size: Some(Vec2::new(100., 4.)),
    //             ..default()
    //         },
    //         transform: Transform::from_xyz(0., 0., 0.),
    //         ..default()
    //     },
    //     RigidBody::Static,
    //     Collider::rectangle(100., 4.),
    // ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.7, 0.7, 0.8),
                custom_size: Some(Vec2::new(1100.0, 50.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -175.0, 0.0),
            ..default()
        },
        RigidBody::Static,
        Collider::rectangle(1100.0, 50.0),
    ));
}
