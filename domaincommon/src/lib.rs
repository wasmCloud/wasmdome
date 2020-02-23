#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate eventsourcing_derive;
pub extern crate eventsourcing;

pub mod commands;
pub mod events;
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

    pub fn adjacent_points(&self, board: &GameBoard) -> Vec<Point> {
        let mut points = Vec::new();
        for direction in &ALL_DIRECTIONS {
            if let Some(target) = self.relative_point(board, direction) {
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
    ) -> Vec<Point> {
        let mut points = Vec::new();
        let mut p = Self::relative_point(&self, board, direction);
        for _i in 0..count {
            match p {
                Some(point) => {
                    points.push(point.clone());
                    p = Self::relative_point(&point, board, direction);
                }
                None => break,
            }
        }
        points
    }

    /// Returns a point 1 unit away in the direction indicated. Grid origin is the most Southwest point
    pub fn relative_point(&self, board: &GameBoard, direction: &GridDirection) -> Option<Point> {
        let destination = match direction {
            GridDirection::North => Point {
                x: self.x,
                y: self.y + 1,
            },
            GridDirection::NorthEast => Point {
                x: self.x + 1,
                y: self.y + 1,
            },
            GridDirection::East => Point {
                x: self.x + 1,
                y: self.y,
            },
            GridDirection::SouthEast => Point {
                x: self.x + 1,
                y: self.y - 1,
            },
            GridDirection::South => Point {
                x: self.x,
                y: self.y - 1,
            },
            GridDirection::SouthWest => Point {
                x: self.x - 1,
                y: self.y - 1,
            },
            GridDirection::West => Point {
                x: self.x - 1,
                y: self.y,
            },
            GridDirection::NorthWest => Point {
                x: self.x - 1,
                y: self.y + 1,
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

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DamageSource {
    Wall,
    Mech(String),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gather_points() {
        let board = GameBoard::default();
        let origin = Point::new(6, 6);

        let points = origin.gather_points(&board, &GridDirection::NorthEast, 6);
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

        let truncated_points = origin.gather_points(&board, &GridDirection::SouthWest, 20);
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
    fn test_adjacent_points() {
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
}
