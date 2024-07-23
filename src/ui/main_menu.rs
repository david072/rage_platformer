use bevy::{color::palettes::css::*, prelude::*};

use super::*;
use crate::GameState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(
                Update,
                (
                    button_interaction::<PlayButton>.pipe(play_button_system),
                    button_interaction::<QuitButton>.pipe(quit_button_system),
                )
                    .run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu);
    }
}

#[derive(Component)]
struct MainMenu;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct QuitButton;

fn setup_main_menu(mut commands: Commands) {
    spawn_root_node(&mut commands)
        .insert(MainMenu)
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Rage Platformer",
                TextStyle {
                    font_size: 50.,
                    color: WHITE.into(),
                    ..default()
                },
            ));
            spawn_sized_box(parent, Val::DEFAULT, Val::Px(50.));
            spawn_button(parent, "Play").insert(PlayButton);
            spawn_sized_box(parent, Val::DEFAULT, Val::Px(20.));
            spawn_button(parent, "Quit").insert(QuitButton);
        });
}

fn cleanup_main_menu(mut commands: Commands, entities: Query<Entity, With<MainMenu>>) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
}

fn play_button_system(In(released): In<bool>, mut next_game_state: ResMut<NextState<GameState>>) {
    if released {
        next_game_state.set(GameState::level(0));
    }
}

fn quit_button_system(In(released): In<bool>, mut exit: EventWriter<AppExit>) {
    if released {
        exit.send(AppExit::Success);
    }
}
