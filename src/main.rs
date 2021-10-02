use std::cmp::max;

use bevy::core::FixedTimestep;
use bevy::prelude::*;

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
// endregion: Resource

// region: Component
struct ActiveBlock {
  direction: Direction,
}
struct StackedBlock;
#[derive(PartialEq)]
struct Position {
  x: i32,
  y: i32,
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

// region: Event
struct RespawnEvent;

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum Label {
  Input,
  Movement,
  Stack,
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
    .add_event::<RespawnEvent>()
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
        .with_system(
          block_free_fall // 自由落下システムを追加する
            .system()
            .label(Label::Movement),
        )
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
        .with_system(respawn_block.system().after(Label::Stack)),
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

fn spawn_block(mut commands: Commands, materials: Res<Materials>) {
  commands
    .spawn_bundle(SpriteBundle {
      material: materials.gray_block.clone(),
      sprite: Sprite::new(Vec2::new(10.0, 10.0)),
      ..Default::default()
    })
    .insert(ActiveBlock {
      direction: Direction::Neutral,
    })
    .insert(Position { x: 3, y: 20 })
    .insert(Size::square(0.8));
}

fn respawn_block(
  commands: Commands,
  materials: Res<Materials>,
  mut reader: EventReader<RespawnEvent>,
) {
  if reader.iter().next().is_some() {
    spawn_block(commands, materials);
  }
}

fn block_movement_input(
  keyboard_input: Res<Input<KeyCode>>,
  mut active_block_query: Query<(&Position, &mut ActiveBlock)>,
  stacked_block_query: Query<&Position, With<StackedBlock>>,
) {
  if let Ok((active_block_position, mut active_block)) = active_block_query.single_mut() {
    let is_collision = |active_block_position: &Position| -> bool {
      stacked_block_query
        .iter()
        .any(|pos| pos == active_block_position)
    };

    let dir: Direction = if keyboard_input.just_pressed(KeyCode::Left) {
      let pos = Position {
        x: active_block_position.x - 1,
        y: active_block_position.y,
      };
      if !is_collision(&pos) {
        Direction::Left
      } else {
        Direction::Neutral
      }
    } else if keyboard_input.just_pressed(KeyCode::Right) {
      let pos = Position {
        x: active_block_position.x + 1,
        y: active_block_position.y,
      };
      if !is_collision(&pos) {
        Direction::Right
      } else {
        Direction::Neutral
      }
    } else if keyboard_input.pressed(KeyCode::Down) {
      let pos = Position {
        x: active_block_position.x,
        y: active_block_position.y - 1,
      };
      if !is_collision(&pos) {
        Direction::Down
      } else {
        Direction::Neutral
      }
    } else if keyboard_input.just_pressed(KeyCode::Up) {
      Direction::Up
    } else {
      Direction::Neutral
    };
    active_block.direction = dir;
  }
}

fn block_free_fall(mut query: Query<&mut Position, With<ActiveBlock>>) {
  if let Ok(mut position) = query.single_mut() {
    position.y -= 1
  }
}

fn block_movement(mut active_block_position: Query<(&mut Position, &ActiveBlock)>) {
  if let Ok((mut pos, active_block)) = active_block_position.single_mut() {
    if active_block.direction == Direction::Left && pos.x > 0 {
      pos.x -= 1;
    } else if active_block.direction == Direction::Right && pos.x < (ARENA_WIDTH - 1) as i32 {
      pos.x += 1;
    } else if active_block.direction == Direction::Down && pos.y > 0 {
      // 急降下
      pos.y -= 1;
    } else if active_block.direction == Direction::Up && pos.y < (ARENA_HEIGHT - 1) as i32 {
      pos.y += 1;
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
  mut writer: EventWriter<RespawnEvent>,
  active_block_query: Query<(Entity, &Position), With<ActiveBlock>>,
  stacked_block_query: Query<&Position, With<StackedBlock>>,
) {
  if let Ok((entity, active_block_position)) = active_block_query.single() {
    let mut stack = |x: i32, y: i32| {
      // despawn active block
      commands.entity(entity).despawn();

      // spawn stacked block
      commands
        .spawn_bundle(SpriteBundle {
          material: materials.white_block.clone(),
          ..Default::default()
        })
        .insert(StackedBlock)
        .insert(Position { x, y })
        .insert(Size::square(0.8));
      writer.send(RespawnEvent);
    };

    if active_block_position.y <= 0 {
      stack(active_block_position.x, max(active_block_position.y, 0));
      return;
    }

    for stacked_block_position in stacked_block_query.iter() {
      if active_block_position.y - 1 == stacked_block_position.y
        && active_block_position.x == stacked_block_position.x
      {
        stack(stacked_block_position.x, stacked_block_position.y + 1);
        break;
      }
    }
  }
}
