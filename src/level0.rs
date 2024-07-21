use avian2d::prelude::*;
use bevy::prelude::*;

fn platform(mut commands: Commands, pos: (f32, f32), size: (f32, f32)) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(1., 1., 1.),
                custom_size: Some(Vec2::new(size.0, size.1)),
                ..default()
            },
            transform: Transform::from_xyz(pos.0, pos.1, 0.),
            ..default()
        },
        RigidBody::Static,
        Collider::rectangle(size.0, size.1),
    ));
}

pub fn setup(commands: Commands) {
    platform(commands, (0., 0.), (1000., 4.));
}
