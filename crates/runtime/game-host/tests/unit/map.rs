use map_project::Collision;
use world_application::{Direction, Position, WorldApplication, WorldCommand, WorldEvent};

use super::load_map;

#[test]
fn checked_in_verdant_route_loads_as_one_playable_world() {
    let map = load_map().unwrap();
    assert_eq!(map.project.format_version, map_project::FORMAT_VERSION);
    assert_eq!(map.project.id.as_str(), "verdant-route");
    assert_eq!((map.project.width, map.project.height), (288, 168));
    assert_eq!(
        map.project.player_spawn,
        map_project::TilePosition::new(108, 84)
    );
    assert_eq!(map.project.actors.len(), 4);
    assert!(map.project.collision_cells[84 * 288 + 71] == Collision::Walkable);
    assert!(map.project.collision_cells[84 * 288 + 72] == Collision::Walkable);
}

#[test]
fn player_moves_across_the_western_meadow_to_crossroads_seam() {
    let mut map = load_map().unwrap().project;
    map.player_spawn = map_project::TilePosition::new(71, 84);
    let world = WorldApplication::from_map_project(&map).unwrap();
    let (world, outcome) = world.transition(WorldCommand::Move(Direction::Right));
    assert!(matches!(outcome.event(), WorldEvent::Moved { .. }));
    assert_eq!(world.player(), Position::new(72, 84));
}
