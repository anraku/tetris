use super::*;

#[test]
fn test_generate_tetorimino_positions() {
  assert_eq!(
    vec![
      Position { x: 0, y: 0 },
      Position { x: 1, y: 0 },
      Position { x: 0, y: 1 },
      Position { x: 1, y: 1 }
    ],
    generate_tetorimino_positions(&Position { x: 0, y: 0 }, &arr2(&[[1, 1], [1, 1]]))
  );
}
