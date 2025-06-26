/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use bevy_ecs::prelude::*;
use bevy_math::{UVec2, Vec2};
use pc_keyboard::KeyCode;

use crate::{
    arch::keyboard::KeyboardState,
    assets::{FLAPPY_BIRD_DATA, FLAPPY_BIRD_SIZE, PIPE_DATA, PIPE_SIZE},
    game::{MenuState, StateScoped, get_random},
    utils::fb::Framebuffer,
};

use super::ecs::*;

#[derive(Component)]
pub struct Player;

#[derive(Resource, Default)]
pub struct Score {
    pub current: u32,
    pub high: u32,
}

#[derive(Component)]
pub struct ScoreText;

pub fn player_setup(mut commands: Commands) {
    commands.spawn((
        Text::new("SCORE - 0\nHIGH SCORE - 0").with_shadow(UVec2::new(1, 1), 0xABABAB),
        Transform::from_translation(UVec2::new(5, 5).as_vec2()),
        ScoreText,
        StateScoped(MenuState::Main),
    ));

    commands.spawn((
        Transform::from_translation(Vec2::new(50.0, 50.0)),
        Velocity::linear(Vec2::ZERO),
        Collider::new(FLAPPY_BIRD_SIZE),
        Sprite::new(*FLAPPY_BIRD_DATA, FLAPPY_BIRD_SIZE),
        RigidBody::Dynamic,
        Player,
        StateScoped(MenuState::Playing),
    ));
}

pub fn player_update(
    mut commands: Commands,
    player: Single<(&mut Transform, &mut Velocity), With<Player>>,
    keyboard_state: Res<KeyboardState>,
    time: Res<Time>,
    fb: Res<Framebuffer>,
    mut last_time: Local<u64>,
) {
    let (mut transform, mut velocity) = player.into_inner();
    if keyboard_state.just_pressed(KeyCode::Spacebar) {
        velocity.linear.y = -200.0;
        transform.rotation = -35.0_f32.to_radians();
    }

    if *last_time + 1_000_000_000 < time.elapsed_ns {
        let quarter = fb.size.y / 4;
        commands.spawn((
            Transform::from_translation(Vec2::new(
                600.0,
                get_random(quarter..(quarter * 3)) as f32,
            )),
            Velocity::linear(Vec2::NEG_X * 200.0),
            Collider::new(PIPE_SIZE),
            Sprite::new(*PIPE_DATA, PIPE_SIZE),
            RigidBody::Static,
            StateScoped(MenuState::Playing),
        ));
        *last_time = time.elapsed_ns;
    }
}

pub fn update_score(mut text: Single<&mut Text, With<ScoreText>>, score: Res<Score>) {
    text.text = alloc::format!("SCORE - {}\nHIGH SCORE - {}", score.current, score.high);
}

pub fn game_over(mut commands: Commands, fb: Res<Framebuffer>, mut score: ResMut<Score>) {
    score.high = score.high.max(score.current);

    let s = "GAME OVER";

    commands.spawn((
        Text::new(s).with_shadow(UVec2::new(2, 2), 0xABABAB),
        Transform::from_translation(
            UVec2::new(
                fb.centered_str_x(s, 2.0),
                fb.centered_str_y(2.0) - fb.font_height * 3,
            )
            .as_vec2(),
        )
        .with_scale(Vec2::splat(2.0)),
        StateScoped(MenuState::GameOver),
    ));

    let s = &alloc::format!(
        "CURRENT SCORE - {}, HIGH SCORE - {}",
        score.current,
        score.high
    );

    commands.spawn((
        Text::new(s).with_shadow(UVec2::new(2, 2), 0xABABAB),
        Transform::from_translation(
            UVec2::new(fb.centered_str_x(s, 2.0), fb.centered_str_y(2.0)).as_vec2(),
        )
        .with_scale(Vec2::splat(2.0)),
        StateScoped(MenuState::GameOver),
    ));

    let s = "PRESS SPACE TO RESTART";

    commands.spawn((
        Text::new(s).with_shadow(UVec2::new(2, 2), 0xABABAB),
        Transform::from_translation(
            UVec2::new(
                fb.centered_str_x(s, 2.0),
                fb.centered_str_y(2.0) + fb.font_height * 3,
            )
            .as_vec2(),
        )
        .with_scale(Vec2::splat(2.0)),
        StateScoped(MenuState::GameOver),
    ));

    score.current = 0;
}
