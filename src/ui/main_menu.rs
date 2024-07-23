use bevy::{color::palettes::css::*, prelude::*};

use super::*;
use crate::GameState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(level_select_menu::LevelSelectMenuPlugin)
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                (
                    button_interaction::<PlayButton>.pipe(play_button_system),
                    button_interaction::<LevelSelectButton>.pipe(level_select_button_system),
                    button_interaction::<QuitButton>.pipe(quit_button_system),
                )
                    .run_if(in_state(GameState::MainMenu)),
            );
    }
}

#[derive(Component)]
struct MainMenu;

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct LevelSelectButton;

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
            spawn_button(parent, "Select Level").insert(LevelSelectButton);
            spawn_sized_box(parent, Val::DEFAULT, Val::Px(20.));
            spawn_button(parent, "Quit").insert(QuitButton);
        });
}

fn cleanup_main_menu(mut commands: Commands, entities: Query<Entity, With<MainMenu>>) {
    for entity in &entities {
        commands.entity(entity).despawn_recursive();
    }
}

fn play_button_system(
    In(released): In<ButtonInteractionResult>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if released.is_some() {
        next_game_state.set(GameState::level(0));
    }
}

fn level_select_button_system(
    In(released): In<ButtonInteractionResult>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if released.is_some() {
        next_game_state.set(GameState::LevelSelect);
    }
}

fn quit_button_system(In(released): In<ButtonInteractionResult>, mut exit: EventWriter<AppExit>) {
    if released.is_some() {
        exit.send(AppExit::Success);
    }
}

mod level_select_menu {
    use super::super::*;
    use crate::levels::LevelGenerator;
    use crate::GameState;

    #[derive(Resource)]
    pub struct LevelSelectPage(u16);

    pub struct LevelSelectMenuPlugin;

    impl Plugin for LevelSelectMenuPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(LevelSelectPage(0))
                .add_systems(OnEnter(GameState::LevelSelect), setup_menu)
                .add_systems(OnExit(GameState::LevelSelect), cleanup_menu)
                .add_systems(
                    Update,
                    (
                        button_interaction::<BackButton>.pipe(back_button_system),
                        button_interaction::<ArrowButton>.pipe(arrow_button_system),
                        button_interaction::<LevelButton>.pipe(level_button_system),
                    )
                        .run_if(in_state(GameState::LevelSelect)),
                );
        }
    }

    #[derive(Component)]
    pub struct LevelSelectMenu;

    #[derive(Component)]
    pub struct BackButton;

    #[derive(Component)]
    pub enum ArrowButton {
        Forward,
        Backward,
    }

    #[derive(Component)]
    pub struct LevelButton(u16);

    pub fn setup_menu(mut commands: Commands) {
        spawn_root_node(&mut commands)
            .insert(LevelSelectMenu)
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(60.),
                            height: Val::Percent(60.),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Stretch,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|parent| {
                        fn button(parent: &mut ChildBuilder, idx: u16) {
                            parent
                                .spawn((
                                    ButtonBundle {
                                        style: Style {
                                            width: Val::Percent(100.),
                                            padding: BUTTON_PADDING,
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        background_color: BLACK.into(),
                                        ..default()
                                    },
                                    LevelButton(idx),
                                ))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle::from_section(
                                        format!("Level {}", idx + 1),
                                        TextStyle {
                                            font_size: 30.,
                                            color: WHITE.into(),
                                            ..default()
                                        },
                                    ));
                                });
                        }

                        for i in 0..3 {
                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Percent(100.),
                                        flex_direction: FlexDirection::Column,
                                        justify_content: JustifyContent::SpaceBetween,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    ..default()
                                })
                                .with_children(|parent| {
                                    for j in 0..3 {
                                        let level_idx = j * 3 + i;
                                        if level_idx >= LevelGenerator::level_count() {
                                            spawn_sized_box(
                                                parent,
                                                Val::Percent(100.),
                                                Val::Px(50.),
                                            );
                                        } else {
                                            button(parent, j * 3 + i);
                                        }
                                    }
                                });

                            if i < 2 {
                                spawn_sized_box(parent, Val::Px(100.), Val::DEFAULT);
                            }
                        }
                    });

                spawn_sized_box(parent, Val::DEFAULT, Val::Px(100.));

                parent
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(50.),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|parent| {
                        fn arrow_button<'a>(
                            parent: &'a mut ChildBuilder,
                            text: impl Into<String>,
                        ) -> EntityCommands<'a> {
                            let mut cmds = parent.spawn(ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
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

                        arrow_button(parent, "<").insert(ArrowButton::Backward);
                        spawn_sized_box(parent, Val::Px(10.), Val::DEFAULT);
                        spawn_button(parent, "Back").insert(BackButton);
                        spawn_sized_box(parent, Val::Px(10.), Val::DEFAULT);
                        arrow_button(parent, ">").insert(ArrowButton::Forward);
                    });
            });
    }

    pub fn cleanup_menu(mut commands: Commands, entities: Query<Entity, With<LevelSelectMenu>>) {
        for entity in &entities {
            commands.entity(entity).despawn_recursive();
        }
    }

    pub fn back_button_system(
        In(released): In<ButtonInteractionResult>,
        mut next_game_state: ResMut<NextState<GameState>>,
    ) {
        if released.is_some() {
            next_game_state.set(GameState::MainMenu);
        }
    }

    pub fn arrow_button_system(
        In(released): In<ButtonInteractionResult>,
        mut page: ResMut<LevelSelectPage>,
        arrow_buttons: Query<(Entity, &ArrowButton)>,
        mut level_buttons: Query<(&mut LevelButton, &Children)>,
        mut texts: Query<&mut Text>,
    ) {
        let Some(released) = released else {
            return;
        };

        let page_delta: i16 = 'l: {
            for (entity, button) in &arrow_buttons {
                if entity != released {
                    continue;
                }

                match button {
                    ArrowButton::Forward => {
                        if (page.0 + 1) * 9 >= LevelGenerator::level_count() {
                            return;
                        }

                        page.0 += 1;
                        break 'l 1;
                    }
                    ArrowButton::Backward => {
                        if page.0 == 0 {
                            return;
                        }

                        page.0 -= 1;
                        break 'l -1;
                    }
                }
            }
            unreachable!();
        };

        for (mut button, children) in &mut level_buttons {
            let mut text = texts.get_mut(*children.first().unwrap()).unwrap();
            if page_delta > 0 {
                button.0 += 9;
            } else {
                button.0 -= 9;
            }

            text.sections[0].value = format!("Level {}", button.0 + 1);
        }
    }

    pub fn level_button_system(
        In(released): In<ButtonInteractionResult>,
        level_buttons: Query<(Entity, &LevelButton)>,
        mut next_game_state: ResMut<NextState<GameState>>,
    ) {
        let Some(released) = released else {
            return;
        };

        for (entity, button) in &level_buttons {
            if entity != released {
                continue;
            }

            next_game_state.set(GameState::level(button.0));
            return;
        }
    }
}
