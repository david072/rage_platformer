use avian2d::math::{Scalar, Vector};
use avian2d::prelude::*;
use bevy::prelude::*;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>().add_systems(
            Update,
            (
                (keyboard_input, update_grounded, update_ducking),
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
pub struct Ducking;

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
                .with_max_time_of_impact(1.),
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

fn update_ducking(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Collider, Has<Ducking>), With<CharacterController>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    spatial_query: SpatialQuery,
) {
    let duck_keys = [KeyCode::ShiftLeft, KeyCode::ShiftRight, KeyCode::ArrowDown];
    for (controller, mut transform, collider, is_ducking) in &mut query {
        // maybe this is cool because the calculation is lazy and shit but idk
        let height = |transform: &Mut<Transform>| {
            collider
                .aabb(transform.translation.xy(), transform.rotation)
                .size()
                .y
        };
        if keyboard_input.any_pressed(duck_keys) {
            if !is_ducking {
                commands.entity(controller).insert(Ducking);
                transform.scale = Vec3::new(1., 0.5, 1.);
                transform.translation.y -= height(&transform) / 4.;
            }
        } else if is_ducking {
            // scale the collider down slightly to allow the player to stand up, even if they are
            // ducking right next to a collider
            let mut cast_collider = collider.clone();
            cast_collider.set_scale(cast_collider.scale() * 0.99, 10);
            // make sure there is enough space above the player to stand up
            let hits = spatial_query.cast_shape(
                &cast_collider,
                // (add 1 to the Y-coord to prevent false collisions due to the smaller collider)
                transform.translation.xy() + Vector::new(0., 1.),
                0.,
                Dir2::Y,
                height(&transform),
                true,
                SpatialQueryFilter::default(),
            );
            if hits.is_some() {
                continue;
            }
            commands.entity(controller).remove::<Ducking>();
            transform.scale = Vec3::ONE;
            // since we're currently ducked, the height is already 0.5x the normal player height
            transform.translation.y += height(&transform) / 2.;
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
        Has<Ducking>,
    )>,
) {
    for event in movement_event_reader.read() {
        for (speed, jump_impulse, mut velocity, is_grounded, is_ducking) in &mut controllers {
            match event {
                MovementAction::Move(direction) => {
                    velocity.x = *direction * speed.0 * time.delta_seconds()
                }
                MovementAction::Jump => {
                    if is_grounded && !is_ducking {
                        velocity.y = jump_impulse.0;
                    }
                }
            }
        }
    }
}
