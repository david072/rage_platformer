use avian2d::math::{Scalar, Vector};
use avian2d::prelude::*;
use bevy::prelude::*;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>().add_systems(
            Update,
            (
                keyboard_input,
                update_grounded,
                movement,
                // apply_movement_damping,
            )
                .chain(),
        );
    }
}

#[derive(Event)]
pub enum MovementAction {
    Move(Scalar),
    Jump,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component)]
pub struct CharacterController;

#[derive(Component)]
pub struct MovementSpeed(Scalar);

#[derive(Component)]
pub struct JumpImpulse(Scalar);

/// The maximum angle a slope can have for the character controller to be able to climb and jump.
/// If the slope is steeper than this angle, the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(Scalar);

#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementSpeed,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(speed: Scalar, jump_impulse: Scalar, max_slope_angle: Scalar) -> Self {
        Self {
            acceleration: MovementSpeed(speed),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(15000.0, 400.0, (30.0 as Scalar).to_radians())
    }
}

#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    rigid_body: RigidBody,
    collider: Collider,
    ground_caster: ShapeCaster,
    locked_axes: LockedAxes,
    movement: MovementBundle,
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider) -> Self {
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController,
            rigid_body: RigidBody::Dynamic,
            collider,
            ground_caster: ShapeCaster::new(caster_shape, Vector::ZERO, 0., Dir2::NEG_Y)
                .with_max_time_of_impact(10.0),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            movement: MovementBundle::default(),
        }
    }
}

fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    movement_event_writer.send(MovementAction::Move(horizontal as Scalar));

    if keyboard_input.any_pressed([KeyCode::Space, KeyCode::ArrowUp]) {
        movement_event_writer.send(MovementAction::Jump);
    }
}

fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>),
        With<CharacterController>,
    >,
) {
    for (entity, hits, rotation, max_slope_angle) in &mut query {
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<(
        &MovementSpeed,
        &JumpImpulse,
        &mut LinearVelocity,
        Has<Grounded>,
    )>,
) {
    for event in movement_event_reader.read() {
        for (speed, jump_impulse, mut velocity, is_grounded) in &mut controllers {
            match event {
                MovementAction::Move(direction) => {
                    velocity.x = *direction * speed.0 * time.delta_seconds()
                }
                MovementAction::Jump => {
                    if is_grounded {
                        velocity.y = jump_impulse.0;
                    }
                }
            }
        }
    }
}
