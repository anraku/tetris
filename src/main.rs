mod main_test;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::hash::Hash;

use bevy::core::FixedTimestep;
use bevy::prelude::*;
use ndarray::prelude::*;
use rand::prelude::random;

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 20;
const BLOCK_RESPAWN_DELAY: f64 = 1.;

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
struct ActiveBlock {
  is_on: bool,
  direction: Direction,
  position: Position,
  block_idx: u32,
}
struct StackTime(f64);
// endregion: Resource

// region: Component
struct PrimitiveBlock {}
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
  Transpose,
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
    .insert_resource(ActiveBlock {
      is_on: false,
      direction: Direction::Neutral,
      position: Position {
        x: 3,
        y: ARENA_WIDTH as i32 - 1,
      },
      block_idx: 0,
    })
    .insert_resource(StackTime(0.))
    .add_startup_system(setup.system())
    .add_startup_stage("game_setup", SystemStage::single(spawn_block.system()))
    .add_system(
      block_movement_input
        .system()
        .label(Label::Input)
        .before(Label::Movement),
    )
    // .add_sysem(block_transpose.system().label(Label::Transpose))
    .add_system_set(
      SystemSet::new()
        .with_run_criteria(FixedTimestep::step(0.5))
        .with_system(
          block_free_fall
            .system()
            .label(Label::Movement)
            .after(Label::Input)
            .after(Label::Transpose),
        )
        .with_system(
          block_movement
            .system()
            .label(Label::Movement)
            .after(Label::Input)
            .after(Label::Transpose),
        ),
    )
    .add_system(
      stack_block
        .system()
        .label(Label::Stack)
        .after(Label::Movement),
    )
    .add_system(
      destroy_block
        .system()
        .label(Label::Destroy)
        .after(Label::Stack),
    )
    .add_system(respawn_block.system().after(Label::Destroy))
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
    m.insert(1, vec![Position { x: 0, y: 0 }, Position { x: 1, y: 0 }, Position { x: 0, y: 1 }, Position { x: 1, y: 1 }]); // square
    m.insert(2, vec![Position { x: -1, y: 1 }, Position { x: 0, y: 1 }, Position { x: 0, y: 0 }, Position { x: 1, y: 0 }]); // S字
    m.insert(3, vec![Position { x: -1, y: 0 }, Position { x: 0, y: 0 }, Position { x: 0, y: 1 }, Position { x: 1, y: 1 }]); // 逆S字
    m.insert(4, vec![Position { x: 1, y: 0 }, Position { x: 0, y: 0 }, Position { x: 0, y: 1 }, Position { x: 0, y: 2 }]); // L字
    m.insert(5, vec![Position { x: -1, y: 0 }, Position { x: 0, y: 0 }, Position { x: 0, y: 1 }, Position { x: 0, y: 2 }]); // 逆L字
    m.insert(6, vec![Position { x: -1, y: 0 }, Position { x: 0, y: 0 }, Position { x: 1, y: 0 }, Position { x: 0, y: 1 }]); // T字
    m.insert(7, vec![Position { x: 0, y: 0 }, Position { x: 0, y: 1 }, Position { x: 0, y: 2 }, Position { x: 0, y: 3 }]); // I字
    m
  };

  pub static ref TETORIMINO_ARRAY: Vec<Array2<u32>> = {
    vec![
      arr2(&[
        [1,1],
        [1,1],
      ]), // square
      arr2(&[
        [1,1,0],
        [0,1,1],
        [0,0,0],
      ]), // S字
      arr2(&[
        [0,1,1],
        [1,1,0],
        [0,0,0],
      ]), // 逆S字
      arr2(&[
        [0,1,0],
        [0,1,0],
        [0,1,1],
      ]), // L字
      arr2(&[
        [0,1,0],
        [0,1,0],
        [1,1,0],
      ]), // 逆L字
      arr2(&[
        [0,1,0],
        [1,1,1],
        [0,0,0],
      ]), // T字
      arr2(&[
        [0,1,0,0],
        [0,1,0,0],
        [0,1,0,0],
        [0,1,0,0],
      ]), // I字
    ]
  };
}

fn generate_tetorimino_positions(base_position: &Position, block: &Array2<u32>) -> Vec<Position> {
  let mut res = vec![];

  if let Some(s) = block.as_slice() {
    let x_size = block.shape()[1];
    let mut y_idx: usize = 0;
    let mut x_idx: usize = 0;
    for v in s.iter() {
      if *v == 1 {
        res.push(Position {
          x: base_position.x + x_idx as i32,
          y: base_position.y + y_idx as i32,
        });
      }

      x_idx += 1;
      if x_idx >= x_size {
        x_idx = 0;
        y_idx += 1;
      }
    }
  }

  res
}

fn spawn_block(
  mut commands: Commands,
  materials: Res<Materials>,
  mut active_block: ResMut<ActiveBlock>,
) {
  if !active_block.is_on {
    let idx = (random::<f32>() * BLOCKMAP.keys().len() as f32) as u32;
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
          .insert(PrimitiveBlock {})
          .insert(Position {
            x: position.x + base_position_x,
            y: position.y + base_position_y,
          })
          .insert(Size::square(0.8));
      }
    }
    active_block.is_on = true;
  }
}

fn respawn_block(
  commands: Commands,
  materials: Res<Materials>,
  active_block: ResMut<ActiveBlock>,
  time: Res<Time>,
  stack_time: ResMut<StackTime>,
) {
  let now = time.seconds_since_startup();
  if !active_block.is_on && now > stack_time.0 + BLOCK_RESPAWN_DELAY {
    spawn_block(commands, materials, active_block);
  }
}

fn block_movement_input(
  keyboard_input: Res<Input<KeyCode>>,
  mut active_block: ResMut<ActiveBlock>,
) {
  let dir: Direction = if keyboard_input.just_pressed(KeyCode::Left) {
    Direction::Left
  } else if keyboard_input.just_pressed(KeyCode::Right) {
    Direction::Right
  } else if keyboard_input.pressed(KeyCode::Down) {
    // 急降下
    Direction::Down
  } else if keyboard_input.just_pressed(KeyCode::Up) {
    Direction::Up
  } else {
    Direction::Neutral
  };
  active_block.direction = dir;
}

fn move_tetoriminos(mut t: Query<&mut Position, Without<StackedBlock>>, diff: &Position) {
  for mut position in t.iter_mut() {
    position.x += diff.x;
    position.y += diff.y;
  }
}

fn block_free_fall(
  mut query: Query<&mut Position, Without<StackedBlock>>,
  stacked_block_query: Query<(&StackedBlock, &Position), Without<PrimitiveBlock>>,
  active_block: Res<ActiveBlock>,
) {
  if active_block.direction == Direction::Down {
    return;
  }
  let mut collision_flag = false;
  for position in query.iter_mut() {
    let is_collision = |pos: &Position| -> bool {
      stacked_block_query
        .iter()
        .any(|(_, stacked_pos)| stacked_pos == pos)
    };
    let p = Position {
      x: position.x,
      y: position.y - 1,
    };
    if position.y <= 0 && !is_collision(&p) {
      collision_flag = true;
    }
  }
  let p = Position { x: 0, y: -1 };
  if !collision_flag {
    move_tetoriminos(query, &p);
  }
}

fn block_movement(
  mut primitive_block_query: Query<&mut Position, Without<StackedBlock>>,
  active_block: ResMut<ActiveBlock>,
  stacked_block_query: Query<&Position, With<StackedBlock>>,
) {
  let is_collision = |pos: &Position| -> bool {
    stacked_block_query
      .iter()
      .any(|stacked_pos| stacked_pos == pos)
  };
  let direction = active_block.direction;

  let mut collision_flag = false;
  if direction == Direction::Left {
    for primitive_block_position in primitive_block_query.iter_mut() {
      let pos = Position {
        x: primitive_block_position.x - 1,
        y: primitive_block_position.y,
      };
      if is_collision(&pos) || primitive_block_position.x <= 0 {
        collision_flag = true;
      }
    }
  } else if direction == Direction::Right {
    for primitive_block_position in primitive_block_query.iter_mut() {
      let pos = Position {
        x: primitive_block_position.x + 1,
        y: primitive_block_position.y,
      };
      if is_collision(&pos) || primitive_block_position.x >= (ARENA_WIDTH - 1) as i32 {
        collision_flag = true;
      }
    }
  } else if direction == Direction::Down {
    for primitive_block_position in primitive_block_query.iter_mut() {
      let pos = Position {
        x: primitive_block_position.x,
        y: primitive_block_position.y - 1,
      };
      if is_collision(&pos) || primitive_block_position.y <= 0 {
        collision_flag = true;
      }
    }
  }

  if !collision_flag {
    if direction == Direction::Left {
      let diff = Position { x: -1, y: 0 };
      move_tetoriminos(primitive_block_query, &diff);
    } else if direction == Direction::Right {
      let diff = Position { x: 1, y: 0 };
      move_tetoriminos(primitive_block_query, &diff);
    } else if direction == Direction::Down {
      let diff = Position { x: 0, y: -1 };
      move_tetoriminos(primitive_block_query, &diff);
    }
  }
}

// fn block_transpose(
//   keyboard_input: Res<Input<KeyCode>>,
//   mut active_block: ResMut<ActiveBlock>,
//   mut primitive_block_query: Query<&mut Position, With<PrimitiveBlock>>,
// ) {
//   if keyboard_input.pressed(KeyCode::A) {
//     active_block.theta += consts::FRAC_PI_4;
//     for mut position in primitive_block_query.iter_mut() {
//       position.x = (position.x as f64 * -active_block.theta.sin()) as i32;
//       position.y = (position.y as f64 * active_block.theta.cos()) as i32;
//     }
//   } else if keyboard_input.pressed(KeyCode::D) {
//     active_block.theta -= consts::FRAC_PI_4;
//     for mut position in primitive_block_query.iter_mut() {
//       position.x = position.x * active_block.theta.sin() as i32;
//       position.y = position.y * -active_block.theta.cos() as i32;
//     }
//   }
// }

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
  mut active_block: ResMut<ActiveBlock>,
  primitive_block_query: Query<(Entity, &Position), With<PrimitiveBlock>>,
  stacked_block_query: Query<&Position, With<StackedBlock>>,
  time: Res<Time>,
  mut stack_time: ResMut<StackTime>,
) {
  let mut stack = || {
    for (entity, primitive_block_position) in primitive_block_query.iter() {
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
          x: primitive_block_position.x,
          y: primitive_block_position.y,
        })
        .insert(Size::square(0.8));
    }

    active_block.is_on = false;
    stack_time.0 = time.seconds_since_startup();
  };

  let is_collision = |pos: &Position| -> bool {
    stacked_block_query
      .iter()
      .any(|stacked_pos| stacked_pos == pos)
  };

  // いずれかのアクティブブロックが地面に接地
  if primitive_block_query.iter().any(|(_, p)| p.y <= 0) {
    stack();
    return;
  }

  let mut collision_flag = false;
  for (_, primitive_block_position) in primitive_block_query.iter() {
    let position = Position {
      x: primitive_block_position.x,
      y: primitive_block_position.y - 1,
    };
    if is_collision(&position) {
      collision_flag = true
    }
  }

  if collision_flag {
    stack();
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
