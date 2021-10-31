#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::hash::Hash;

use bevy::core::FixedTimestep;
use bevy::prelude::*;
use rand::prelude::random;

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 20;

// region: Resources
struct Materials {
  gray_block: Handle<ColorMaterial>,
  white_block: Handle<ColorMaterial>,
}
struct MainWindow {
  w: u32,
  h: u32,
}
impl Default for MainWindow {
  fn default() -> Self {
    Self { w: 400, h: 800 }
  }
}
struct ExistActiveBlocks(bool);
struct BlockDirection(Direction);
// endregion: Resource

// region: Component
struct ActiveBlock {}
struct StackedBlock;
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Position {
  x: i32,
  y: i32,
}
impl PartialEq<&Position> for Mut<'_, Position> {
  fn eq(&self, other: &&Position) -> bool {
    self.x.eq(&other.x) && self.y.eq(&other.y)
  }
}
impl PartialEq<Mut<'_, Position>> for &Position {
  fn eq(&self, other: &Mut<Position>) -> bool {
    self.x.eq(&other.x) && self.y.eq(&other.y)
  }
}
struct Size {
  width: f32,
  height: f32,
}
impl Size {
  pub fn square(x: f32) -> Self {
    Self {
      width: x,
      height: x,
    }
  }
}
#[derive(PartialEq, Copy, Clone)]
enum Direction {
  Neutral,
  Left,
  Up,
  Right,
  Down,
}
// endregion: Component

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum Label {
  Input,
  Movement,
  Stack,
  Destroy,
}

fn main() {
  App::build()
    .insert_resource(WindowDescriptor {
      title: "Tetris".to_string(),
      width: 400.0,
      height: 800.0,
      ..Default::default()
    }) // Windowの設定
    .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
    .insert_resource(MainWindow::default())
    .insert_resource(ExistActiveBlocks(false))
    .insert_resource(BlockDirection(Direction::Neutral))
    .add_startup_system(setup.system())
    .add_startup_stage("game_setup", SystemStage::single(spawn_block.system()))
    .add_system(
      block_movement_input
        .system()
        .label(Label::Input)
        .before(Label::Movement),
    )
    .add_system_set(
      SystemSet::new()
        .with_run_criteria(FixedTimestep::step(0.5))
        .with_system(block_free_fall.system().label(Label::Movement))
        .with_system(
          block_movement
            .system()
            .label(Label::Movement)
            .after(Label::Input),
        )
        .with_system(
          stack_block
            .system()
            .label(Label::Stack)
            .after(Label::Movement),
        )
        .with_system(
          destroy_block
            .system()
            .label(Label::Destroy)
            .after(Label::Stack),
        )
        .with_system(respawn_block.system().after(Label::Destroy)),
    )
    .add_system(block_movement.system())
    .add_system_set_to_stage(
      CoreStage::PostUpdate,
      SystemSet::new()
        .with_system(position_translation.system())
        .with_system(size_scaling.system()),
    )
    .add_plugins(DefaultPlugins)
    .run();
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
  commands.spawn_bundle(OrthographicCameraBundle::new_2d());
  commands.insert_resource(Materials {
    gray_block: materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
    white_block: materials.add(Color::rgb(0.1, 0.1, 0.1).into()),
  });
}

lazy_static! {
  pub static ref BLOCKMAP: HashMap<u32, Vec<Position>> = {
    let mut m = HashMap::new();
    m.insert(0, vec![Position { x: 0, y: 0 }]); // Block1つだけ
    m.insert(1, vec![Position { x: 0, y: 0 }, Position { x: 1, y: 0 }, Position { x: 0, y: 1 }, Position { x: 1, y: 1 }]); // square
    m.insert(2, vec![Position { x: -1, y: 0 }, Position { x: 0, y: 0 }, Position { x: 0, y: 1 }, Position { x: 1, y: 1 }]); // S字
    m
  };
}

fn spawn_block(
  mut commands: Commands,
  materials: Res<Materials>,
  mut exist_active_blocks: ResMut<ExistActiveBlocks>,
) {
  if !exist_active_blocks.0 {
    let idx = (random::<f32>() * 3 as f32) as u32;
    if let Some(positions) = BLOCKMAP.get(&idx) {
      let base_position_x = 3;
      let base_position_y = (ARENA_HEIGHT - 1) as i32;

      for position in positions.iter() {
        commands
          .spawn_bundle(SpriteBundle {
            material: materials.gray_block.clone(),
            sprite: Sprite::new(Vec2::new(10.0, 10.0)),
            ..Default::default()
          })
          .insert(ActiveBlock {})
          .insert(Position {
            x: position.x + base_position_x,
            y: position.y + base_position_y,
          })
          .insert(Size::square(0.8));
      }
    }
    exist_active_blocks.0 = true;
  }
}

fn respawn_block(
  commands: Commands,
  materials: Res<Materials>,
  exist_active_blocks: ResMut<ExistActiveBlocks>,
) {
  if !exist_active_blocks.0 {
    spawn_block(commands, materials, exist_active_blocks);
  }
}

fn block_movement_input(
  keyboard_input: Res<Input<KeyCode>>,
  mut block_direction: ResMut<BlockDirection>,
) {
  let dir: Direction = if keyboard_input.just_pressed(KeyCode::Left) {
    Direction::Left
  } else if keyboard_input.just_pressed(KeyCode::Right) {
    Direction::Right
  } else if keyboard_input.pressed(KeyCode::Down) {
    Direction::Down
  } else if keyboard_input.just_pressed(KeyCode::Up) {
    Direction::Up
  } else {
    Direction::Neutral
  };
  block_direction.0 = dir;
}

fn block_free_fall(
  mut query: Query<(&ActiveBlock, &mut Position), Without<StackedBlock>>,
  stacked_block_query: Query<(&StackedBlock, &Position), Without<ActiveBlock>>,
) {
  for (_, mut position) in query.iter_mut() {
    let is_collision = |pos: &Position| -> bool {
      stacked_block_query
        .iter()
        .any(|(_, stacked_pos)| stacked_pos == pos)
    };
    let p = Position {
      x: position.x,
      y: position.y - 1,
    };
    if position.y > 0 && !is_collision(&p) {
      position.y -= 1
    }
  }
}

fn block_movement(
  mut active_block_query: Query<&mut Position, Without<StackedBlock>>,
  block_direction: ResMut<BlockDirection>,
  stacked_block_query: Query<&Position, With<StackedBlock>>,
) {
  for mut active_block_position in active_block_query.iter_mut() {
    let is_collision = |pos: &Position| -> bool {
      stacked_block_query
        .iter()
        .any(|stacked_pos| stacked_pos == pos)
    };

    if block_direction.0 == Direction::Left && active_block_position.x > 0 {
      let pos = Position {
        x: active_block_position.x - 1,
        y: active_block_position.y,
      };
      if !is_collision(&pos) {
        active_block_position.x -= 1;
      }
    } else if block_direction.0 == Direction::Right
      && active_block_position.x < (ARENA_WIDTH - 1) as i32
    {
      let pos = Position {
        x: active_block_position.x + 1,
        y: active_block_position.y,
      };
      if !is_collision(&pos) {
        active_block_position.x += 1;
      }
    } else if block_direction.0 == Direction::Down && active_block_position.y > 0 {
      let pos = Position {
        x: active_block_position.x,
        y: active_block_position.y - 1,
      };
      if !is_collision(&pos) {
        // 急降下
        active_block_position.y -= 1;
      }
    } else if block_direction.0 == Direction::Up
      && active_block_position.y < (ARENA_HEIGHT - 1) as i32
    {
      active_block_position.y += 1;
    }
  }
}

fn size_scaling(window: Res<MainWindow>, mut q: Query<(&Size, &mut Sprite)>) {
  for (sprite_size, mut sprite) in q.iter_mut() {
    sprite.size = Vec2::new(
      sprite_size.width / ARENA_WIDTH as f32 * window.w as f32,
      sprite_size.height / ARENA_HEIGHT as f32 * window.h as f32,
    );
  }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
  fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
    let tile_size = bound_window / bound_game;
    pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
  }
  let window = windows.get_primary().unwrap();
  for (pos, mut transform) in q.iter_mut() {
    transform.translation = Vec3::new(
      convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
      convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
      0.0,
    );
  }
}

fn stack_block(
  mut commands: Commands,
  materials: Res<Materials>,
  active_block_query: Query<(Entity, &Position), With<ActiveBlock>>,
  stacked_block_query: Query<&Position, With<StackedBlock>>,
  mut exist_active_blocks: ResMut<ExistActiveBlocks>,
) {
  let mut stack = || {
    for (entity, active_block_position) in active_block_query.iter() {
      // despawn active block
      commands.entity(entity).despawn();

      // spawn stacked block
      commands
        .spawn_bundle(SpriteBundle {
          material: materials.white_block.clone(),
          ..Default::default()
        })
        .insert(StackedBlock)
        .insert(Position {
          x: active_block_position.x,
          y: active_block_position.y,
        })
        .insert(Size::square(0.8));
    }

    exist_active_blocks.0 = false;
  };

  for (_, active_block_position) in active_block_query.iter() {
    if active_block_position.y <= 0 {
      stack();
      return;
    }
    for stacked_block_position in stacked_block_query.iter() {
      if active_block_position.y - 1 == stacked_block_position.y
        && active_block_position.x == stacked_block_position.x
      {
        stack();
        return;
      }
    }
  }
}

fn destroy_block(
  mut commands: Commands,
  mut query: Query<(Entity, &mut Position), With<StackedBlock>>,
) {
  for h in 0..ARENA_HEIGHT {
    let mut entities = Vec::new();
    for (entity, position) in query.iter_mut() {
      if position.y == h as i32 {
        entities.push(entity);
      }
    }
    if entities.len() == ARENA_WIDTH as usize {
      // blocksにあるBlockを削除
      for &entity in entities.iter() {
        commands.entity(entity).despawn();
      }
      // hより高いBlockをすべて高さを-1する
      for (_, mut position) in query.iter_mut() {
        if position.y > h as i32 {
          position.y -= 1;
        }
      }
    } else if entities.len() == 0 {
      return;
    }
  }
}
