use bevy::{color::palettes::css::*, ecs::system::EntityCommands, prelude::*};

pub mod main_menu;
pub mod pause_menu;

const BUTTON_WIDTH: Val = Val::Percent(20.);
const BUTTON_PADDING: UiRect = UiRect::all(Val::Px(10.));
const NORMAL_BUTTON: Srgba = BLACK;
const HOVERED_BUTTON: Srgba = DARK_SLATE_GREY;
const PRESSED_BUTTON: Srgba = GREY;

type ButtonInteractionResult = Option<Entity>;

fn spawn_root_node<'a>(commands: &'a mut Commands) -> EntityCommands<'a> {
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    })
}

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

fn button_interaction<C: Component>(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut interaction_query: Query<
        (Entity, &Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<C>),
    >,
) -> ButtonInteractionResult {
    for (entity, interaction, mut bg) in &mut interaction_query {
        match interaction {
            Interaction::None => *bg = NORMAL_BUTTON.into(),
            Interaction::Hovered => {
                *bg = HOVERED_BUTTON.into();

                if mouse_input.just_released(MouseButton::Left) {
                    return Some(entity);
                }
            }
            Interaction::Pressed => *bg = PRESSED_BUTTON.into(),
        }
    }

    None
}
