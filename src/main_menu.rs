use bevy::{color::palettes::css::*, ecs::system::EntityCommands, prelude::*};

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

const BUTTON_WIDTH: Val = Val::Percent(20.);
const BUTTON_PADDING: UiRect = UiRect::all(Val::Px(10.));
const NORMAL_BUTTON: Srgba = BLACK;
const HOVERED_BUTTON: Srgba = DARK_SLATE_GREY;
const PRESSED_BUTTON: Srgba = GREY;

#[derive(Component)]
struct MainMenu;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct QuitButton;

fn spawn_button<'a>(parent: &'a mut ChildBuilder, text: impl Into<String>) -> EntityCommands<'a> {
    let mut cmds = parent.spawn(ButtonBundle {
        style: Style {
            width: BUTTON_WIDTH,
            padding: BUTTON_PADDING,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        background_color: BLACK.into(),
        ..default()
    });
    cmds.with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            text,
            TextStyle {
                font_size: 30.,
                color: WHITE.into(),
                ..default()
            },
        ));
    });
    cmds
}

fn spawn_sized_box(parent: &mut ChildBuilder, width: Val, height: Val) {
    parent.spawn(NodeBundle {
        style: Style {
            width,
            height,
            ..default()
        },
        ..default()
    });
}

fn setup_main_menu(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            MainMenu,
        ))
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

fn button_interaction<C: Component>(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<C>),
    >,
) -> bool {
    let mut result = false;
    for (interaction, mut bg) in &mut interaction_query {
        match interaction {
            Interaction::None => *bg = NORMAL_BUTTON.into(),
            Interaction::Hovered => {
                *bg = HOVERED_BUTTON.into();

                if mouse_input.just_released(MouseButton::Left) {
                    result = true;
                }
            }
            Interaction::Pressed => *bg = PRESSED_BUTTON.into(),
        }
    }

    return result;
}

fn play_button_system(In(released): In<bool>, mut next_game_state: ResMut<NextState<GameState>>) {
    if released {
        next_game_state.set(GameState::Level(0));
    }
}

fn quit_button_system(In(released): In<bool>, mut exit: EventWriter<AppExit>) {
    if released {
        exit.send(AppExit::Success);
    }
}
