use crate::tile_map::Block;
use crate::tile_map::Goal;
use crate::tile_map::IsMoving;
use crate::tile_map::LevelWalls;
use crate::tile_map::Pushable;
use crate::GameState;
use bevy::prelude::*;
use bevy_ecs_ldtk::{GridCoords, LdtkEntity, LevelSelection};

pub struct PlayerPlugin;

#[derive(Default, PartialEq, Debug)]
pub enum Direction {
    North,
    #[default]
    East,
    South,
    West,
    None,
}

#[derive(Default, Component)]
pub struct Player {
    face_direction: Direction,
}

#[derive(Default, Bundle, LdtkEntity)]
pub struct PlayerBundle {
    player: Player,
    #[sprite_sheet_bundle]
    sprite_bundle: SpriteSheetBundle,
    #[grid_coords]
    grid_coords: GridCoords,
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                move_player_from_input.run_if(in_state(GameState::Playing)),
                turn_player_from_input.run_if(in_state(GameState::Playing)),
                check_goal.run_if(in_state(GameState::Playing)),
                update_player_facing_direction.run_if(in_state(GameState::Playing)),
                grab_from_held_input.run_if(in_state(GameState::Playing)),
                ungrab_from_release_input.run_if(in_state(GameState::Playing)),
                highlight_grabbed.run_if(in_state(GameState::Playing)),
                unhighlight_grabbed.run_if(in_state(GameState::Playing)),
            ),
        );
    }
}

pub fn turn_player_from_input(
    mut player_query: Query<&mut Player, Without<IsMoving>>,
    input: Res<Input<KeyCode>>,
) {
    for mut player in player_query.iter_mut() {
        if input.pressed(KeyCode::W) {
            player.face_direction = Direction::North;
        } else if input.pressed(KeyCode::A) {
            player.face_direction = Direction::West;
        } else if input.pressed(KeyCode::S) {
            player.face_direction = Direction::South;
        } else if input.pressed(KeyCode::D) {
            player.face_direction = Direction::East;
        }
    }
}

fn get_neighbor_direction(origin: GridCoords, neighbor: GridCoords) -> Direction {
    if origin + GridCoords::new(0, 1) == neighbor {
        return Direction::North;
    } else if origin + GridCoords::new(1, 0) == neighbor {
        return Direction::East;
    } else if origin + GridCoords::new(0, -1) == neighbor {
        return Direction::South;
    } else if origin + GridCoords::new(-1, 0) == neighbor {
        return Direction::West;
    }
    return Direction::None;
}

#[derive(Default, Component)]
pub struct Grabbed;

pub fn grab_from_held_input(
    mut commands: Commands,
    player_query: Query<(&Player, &GridCoords), Without<IsMoving>>,
    pushable_query: Query<(Entity, &GridCoords), (Without<IsMoving>, With<Pushable>)>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for (entity, pushable_grid_coords) in pushable_query.iter() {
            for (player, player_grid_coords) in player_query.iter() {
                let neighbor_direction =
                    get_neighbor_direction(*player_grid_coords, *pushable_grid_coords);
                if get_neighbor_direction(*player_grid_coords, *pushable_grid_coords)
                    == player.face_direction
                {
                    commands.entity(entity).insert(Grabbed);
                }
            }
        }
    }
}

pub fn ungrab_from_release_input(
    mut commands: Commands,
    pushable_query: Query<Entity, With<Pushable>>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_released(KeyCode::Space) {
        for entity in pushable_query.iter() {
            commands.entity(entity).remove::<Grabbed>();
        }
    }
}

pub fn update_player_facing_direction(
    mut player_query: Query<(&mut Player, &mut TextureAtlasSprite), Changed<Player>>,
) {
    for (player, mut sprite) in player_query.iter_mut() {
        match player.face_direction {
            Direction::North => sprite.index = 2,
            Direction::East => {
                sprite.index = 1;
                sprite.flip_x = true;
            }
            Direction::South => sprite.index = 0,
            Direction::West => {
                sprite.index = 1;
                sprite.flip_x = false;
            }
            _ => {
                // default to facing right
                sprite.index = 1;
                sprite.flip_x = true;
            }
        }
    }
}

pub fn highlight_grabbed(mut grabbed_query: Query<&mut TextureAtlasSprite, With<Grabbed>>
) {
    for mut sprite in grabbed_query.iter_mut() {
        sprite.color = Color::rgb(0.0, 1.0, 0.0);
    }
}

pub fn unhighlight_grabbed(mut grabbed_query: Query<&mut TextureAtlasSprite, Without<Grabbed>>
) {
    for mut sprite in grabbed_query.iter_mut() {
        sprite.color = Color::rgb(1.0, 1.0, 1.0);
    }
}

pub fn move_player_from_input(
    mut commands: Commands,
    mut player_query: Query<
        (Entity, &mut GridCoords),
        (With<Player>, Without<IsMoving>, Without<Block>),
    >,
    mut block_query: Query<
        (Entity, &mut GridCoords),
        (With<Block>, Without<IsMoving>, Without<Player>),
    >,
    input: Res<Input<KeyCode>>,
    level_walls: Res<LevelWalls>,
) {
    let movement_direction = if input.pressed(KeyCode::W) {
        GridCoords::new(0, 1)
    } else if input.pressed(KeyCode::A) {
        GridCoords::new(-1, 0)
    } else if input.pressed(KeyCode::S) {
        GridCoords::new(0, -1)
    } else if input.pressed(KeyCode::D) {
        GridCoords::new(1, 0)
    } else {
        return;
    };

    let mut blocks: Vec<GridCoords> = Vec::new();
    for (_, block_grid_coords) in block_query.iter() {
        blocks.push(*block_grid_coords);
    }

    for (entity, mut player_grid_coords) in player_query.iter_mut() {
        let player_destination = *player_grid_coords + movement_direction;
        let block_push_destination = *player_grid_coords + movement_direction + movement_direction;
        let block_pull_origin = *player_grid_coords - movement_direction;
        let block_pull_destination = *player_grid_coords;
        let mut hit_block = false;
        let mut hit_second_block = false;
        for (entity, mut cords) in block_query.iter_mut() {
            // PUSH LOGIC
            if *cords == player_destination {
                hit_block = true;
                for block_coords in blocks.iter() {
                    if block_push_destination == *block_coords {
                        hit_second_block = true;
                    }
                }
                if !hit_second_block && !level_walls.in_wall(&block_push_destination) {
                    commands.entity(entity).insert(IsMoving);
                    *cords = block_push_destination;
                }
            }
            // PULL LOGIC
            if *cords == block_pull_origin
                && !level_walls.in_wall(&player_destination)
                && input.pressed(KeyCode::Space)
            {
                if !hit_block && !level_walls.in_wall(&player_destination) {
                    commands.entity(entity).insert(IsMoving);
                    *cords = block_pull_destination;
                }
            }
        }
        // MOVE LOGIC
        if (!hit_block
            || (hit_block && !hit_second_block && !level_walls.in_wall(&block_push_destination)))
            && !level_walls.in_wall(&player_destination)
        {
            commands.entity(entity).insert(IsMoving);
            *player_grid_coords = player_destination;
        }
    }
}

pub fn check_goal(
    level_selection: ResMut<LevelSelection>,
    players: Query<&GridCoords, (With<Player>, Changed<GridCoords>)>,
    goals: Query<&GridCoords, With<Goal>>,
) {
    if players
        .iter()
        .zip(goals.iter())
        .any(|(player_grid_coords, goal_grid_coords)| player_grid_coords == goal_grid_coords)
    {
        let indices = match level_selection.into_inner() {
            LevelSelection::Indices(indices) => indices,
            _ => panic!("level selection should always be Indices in this game"),
        };

        indices.level += 1;
    }
}
