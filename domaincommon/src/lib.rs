use std::collections::HashSet;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate eventsourcing_derive;
pub extern crate eventsourcing;

pub use radar::RadarPing;

pub mod commands;
pub mod events;
pub mod leaderboard;
mod radar;
pub mod state;

pub(crate) const DOMAIN_VERSION: &str = "1.0";

const DEFAULT_BOARD_HEIGHT: u32 = 100;
const DEFAULT_BOARD_WIDTH: u32 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    pub fn bearing(&self, target: &Point) -> GridDirection {
        let dx = (target.x - self.x) as f64;
        let dy = (target.y - self.y) as f64;
        let mut angle = 90.0 - dy.atan2(dx).to_degrees();

        if angle < 0.0 {
            angle = angle + 360.0;
        }

        println!("Angle: {}", angle);
        let idx = angle.trunc() as usize / 45;
        ALL_DIRECTIONS[idx]
    }

    pub fn adjacent_points(&self, board: &GameBoard) -> Vec<Point> {
        let mut points = Vec::new();
        for direction in &ALL_DIRECTIONS {
            if let Some(target) = self.relative_point(board, direction, 1) {
                points.push(target)
            }
        }
        points
    }

    pub fn gather_points(
        &self,
        board: &GameBoard,
        direction: &GridDirection,
        count: usize,
    ) -> Vec<(Point, usize)> {
        let mut points = Vec::new();
        let mut p = Self::relative_point(&self, board, direction, 1);
        for i in 0..count {
            match p {
                Some(point) => {
                    points.push((point.clone(), i + 1));
                    p = Self::relative_point(&point, board, direction, 1);
                }
                None => break,
            }
        }
        points
    }

    /// Returns a point 1 unit away in the direction indicated. Grid origin is the most Southwest point
    pub fn relative_point(
        &self,
        board: &GameBoard,
        direction: &GridDirection,
        length: i32,
    ) -> Option<Point> {
        let destination = match direction {
            GridDirection::North => Point {
                x: self.x,
                y: self.y + length,
            },
            GridDirection::NorthEast => Point {
                x: self.x + length,
                y: self.y + length,
            },
            GridDirection::East => Point {
                x: self.x + length,
                y: self.y,
            },
            GridDirection::SouthEast => Point {
                x: self.x + length,
                y: self.y - length,
            },
            GridDirection::South => Point {
                x: self.x,
                y: self.y - length,
            },
            GridDirection::SouthWest => Point {
                x: self.x - length,
                y: self.y - length,
            },
            GridDirection::West => Point {
                x: self.x - length,
                y: self.y,
            },
            GridDirection::NorthWest => Point {
                x: self.x - length,
                y: self.y + length,
            },
        };
        if !destination.is_on_board(board) {
            None
        } else {
            Some(destination)
        }
    }

    pub fn is_on_board(&self, board: &GameBoard) -> bool {
        self.x <= board.width as _ && self.y <= board.height as _ && self.x >= 0 && self.y >= 0
    }
}

/// Represents the dimensions and other metadata for a game board. All game boards
/// have an origin of (0,0) that starts in the bottom left (southwest) corner of
/// the scene.
#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub struct GameBoard {
    pub width: u32,
    pub height: u32,
}

impl Default for GameBoard {
    fn default() -> Self {
        GameBoard {
            width: DEFAULT_BOARD_WIDTH,
            height: DEFAULT_BOARD_HEIGHT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeaponType {
    Primary = 0,
    Secondary = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum GridDirection {
    North = 0,
    NorthEast = 1,
    East = 2,
    SouthEast = 3,
    South = 4,
    SouthWest = 5,
    West = 6,
    NorthWest = 7,
}

static ALL_DIRECTIONS: [GridDirection; 8] = [
    GridDirection::North,
    GridDirection::NorthEast,
    GridDirection::East,
    GridDirection::SouthEast,
    GridDirection::South,
    GridDirection::SouthWest,
    GridDirection::West,
    GridDirection::NorthWest,
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchParameters {
    pub match_id: String,
    pub width: u32,
    pub height: u32,
    pub actors: Vec<String>,
    pub max_turns: u32,
    pub aps_per_turn: u32,
}

impl MatchParameters {
    pub fn new(
        match_id: String,
        width: u32,
        height: u32,
        max_turns: u32,
        aps_per_turn: u32,
        actors: Vec<String>,
    ) -> Self {
        MatchParameters {
            match_id,
            width,
            height,
            actors,
            max_turns,
            aps_per_turn,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TurnStatus {
    pub current: u32,
    pub taken: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DamageSource {
    Wall,
    MechWeapon(String),
    MechCollision(String),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn gather_points() {
        let board = GameBoard::default();
        let origin = Point::new(6, 6);

        let points = origin
            .gather_points(&board, &GridDirection::NorthEast, 6)
            .into_iter()
            .map(|(p, _d)| p)
            .collect::<Vec<_>>();
        assert_eq!(
            points,
            vec![
                Point::new(7, 7),
                Point::new(8, 8),
                Point::new(9, 9),
                Point::new(10, 10),
                Point::new(11, 11),
                Point::new(12, 12)
            ]
        );

        let truncated_points = origin
            .gather_points(&board, &GridDirection::SouthWest, 20)
            .into_iter()
            .map(|(p, _d)| p)
            .collect::<Vec<_>>();
        assert_eq!(
            truncated_points,
            vec![
                Point::new(5, 5),
                Point::new(4, 4),
                Point::new(3, 3),
                Point::new(2, 2),
                Point::new(1, 1),
                Point::new(0, 0),
            ]
        );
    }

    #[test]
    fn adjacent_points() {
        let board = GameBoard::default();
        let origin = Point::new(10, 5);
        let points = origin.adjacent_points(&board);
        assert_eq!(
            points,
            vec![
                Point::new(10, 6), // North
                Point::new(11, 6),
                Point::new(11, 5), // East
                Point::new(11, 4),
                Point::new(10, 4), // South
                Point::new(9, 4),
                Point::new(9, 5), // West
                Point::new(9, 6),
            ]
        )
    }

    #[test]
    fn compute_bearing() {
        // this should be a 45 degree bearing, or NorthEast
        let me = Point::new(0, 0);
        let them = Point::new(5, 5);
        assert_eq!(me.bearing(&them), GridDirection::NorthEast);

        assert_eq!(
            Point::new(0, 0).bearing(&Point::new(5, 0)),
            GridDirection::East
        );
        assert_eq!(
            Point::new(0, 0).bearing(&Point::new(0, -5)),
            GridDirection::South
        );
        assert_eq!(
            Point::new(1, 1).bearing(&Point::new(-1, -1)),
            GridDirection::SouthWest
        );
        assert_eq!(
            Point::new(5, 10).bearing(&Point::new(1, 6)),
            GridDirection::SouthWest
        );
        assert_eq!(
            Point::new(0, 0).bearing(&Point::new(0, 5)),
            GridDirection::North
        );
        assert_eq!(
            Point::new(6, 8).bearing(&Point::new(10, 4)),
            GridDirection::SouthEast
        );
        assert_eq!(
            Point::new(9, 11).bearing(&Point::new(4, 11)),
            GridDirection::West
        );
    }
}
