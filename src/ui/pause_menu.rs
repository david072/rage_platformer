use bevy::prelude::*;

use super::*;
use crate::{GameState, IsPaused};

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(IsPaused::Paused), setup_pause_menu)
            .add_systems(OnExit(IsPaused::Paused), cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    button_interaction::<ResumeButton>.pipe(resume_button_system),
                    button_interaction::<ExitToMenuButton>.pipe(exit_to_main_menu_button_system),
                )
                    .run_if(in_state(IsPaused::Paused)),
            );
    }
}

#[derive(Component)]
struct PauseMenu;

#[derive(Component)]
struct ResumeButton;

#[derive(Component)]
struct ExitToMenuButton;

fn setup_pause_menu(mut commands: Commands) {
    spawn_root_node(&mut commands)
        .insert(PauseMenu)
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BLACK.with_alpha(0.5).into(),
                    ..default()
                })
                .with_children(|parent| {
                    spawn_button(parent, "Resume").insert(ResumeButton);
                    spawn_sized_box(parent, Val::DEFAULT, Val::Px(20.));
                    spawn_button(parent, "Exit to Menu").insert(ExitToMenuButton);
                });
        });
}

fn cleanup_pause_menu(mut commands: Commands, entities: Query<Entity, With<PauseMenu>>) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
}

fn resume_button_system(
    In(released): In<bool>,
    game_state: Res<State<GameState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if !released {
        return;
    }

    let GameState::Level { index, .. } = **game_state else {
        return;
    };
    next_game_state.set(GameState::Level {
        index,
        paused: false,
    });
}

fn exit_to_main_menu_button_system(
    In(released): In<bool>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if !released {
        return;
    }

    next_game_state.set(GameState::MainMenu);
}
