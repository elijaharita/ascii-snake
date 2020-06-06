extern crate crossterm;
extern crate rand;

use crossterm::{cursor, terminal, QueueableCommand};
use rand::{prelude::*, thread_rng};
use std::io::{prelude::*, stdin, stdout};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

type SnakeVal = i32;

fn main() {
    use std::time::{Duration, Instant};

    // Create game
    let mut game = Game::new(16, 16);

    // Start alternate terminal view and disable cursor to prepare for drawing
    stdout()
        .queue(terminal::EnterAlternateScreen)
        .unwrap()
        .queue(cursor::Hide)
        .unwrap()
        .flush()
        .unwrap();

    terminal::enable_raw_mode().unwrap();

    // Game loop timing information
    let tick_rate: f32 = 10.0;
    let mut last_game_update = Instant::now();

    // Spawn control input channel
    let input_channel = spawn_input_channel();
    let mut direction_input = Direction::Up;

    // Game loop
    loop {
        // Process input
        if let Ok(direction) = input_channel.try_recv() {
            direction_input = direction;
        }

        // If the fixed time step has passed, perform the next update
        let now = Instant::now();
        if now - last_game_update > Duration::from_secs_f32(1.0 / tick_rate) {
            last_game_update = now;

            // Set the direction to the latest input
            let _ = game.set_direction(direction_input);

            // Update
            game.update();

            // Clear terminal and render
            stdout()
                .queue(terminal::Clear(terminal::ClearType::All))
                .unwrap()
                .queue(cursor::MoveTo(0, 0))
                .unwrap();
            game.render_ascii();

            // Stop running the game loop if the player died
            if !game.alive() {
                println!("You died!");
                std::thread::sleep(Duration::from_secs(1));
                break;
            }
        }
    }

    // Reset terminal to original state
    stdout()
        .queue(terminal::LeaveAlternateScreen)
        .unwrap()
        .queue(cursor::Show)
        .unwrap()
        .flush()
        .unwrap();

    terminal::disable_raw_mode().unwrap();
}

fn spawn_input_channel() -> Receiver<Direction> {
    let (tx, rx) = channel::<Direction>();

    thread::spawn(move || loop {
        let mut buf = [0u8; 1];
        stdin().read_exact(&mut buf).unwrap();
        tx.send(match buf[0] as char {
            'w' => Direction::Up,
            's' => Direction::Down,
            'a' => Direction::Left,
            'd' => Direction::Right,
            _ => continue,
        })
        .unwrap();
    });

    rx
}

// The board containing the snake and food
struct Game {
    width: i32,
    height: i32,
    tiles: Vec<Vec<Tile>>, // tiles[x][y]
    direction: Direction,
    alive: bool,
    length: i32,
    head_x: i32,
    head_y: i32,
}

impl Game {
    // Create a world with the specified size
    fn new(width: i32, height: i32) -> Self {
        let mut new = Self {
            width,
            height,
            tiles: vec![vec![Tile::Empty; height as usize]; width as usize],
            direction: Direction::Up,
            alive: true,
            length: 3,
            head_x: width / 2,
            head_y: height / 2,
        };

        new.spawn_food();

        new
    }

    // Set the snake's direction
    // Returns an error if direction is opposite to current direction
    fn set_direction(&mut self, direction: Direction) -> Result<(), ()> {
        if direction == self.direction.opposite() {
            Err(())
        } else {
            self.direction = direction;
            Ok(())
        }
    }

    fn alive(&self) -> bool {
        self.alive
    }

    fn update(&mut self) {
        // Move head
        match self.direction {
            Direction::Up => self.head_y -= 1,
            Direction::Down => self.head_y += 1,
            Direction::Left => self.head_x -= 1,
            Direction::Right => self.head_x += 1,
        }

        // Check for out of bounds
        if self.head_x < 0
            || self.head_x >= self.width
            || self.head_y < 0
            || self.head_y >= self.height
        {
            // Die if out of bounds
            self.alive = false;
            return;
        }

        // Check for collision
        match self.tiles[self.head_x as usize][self.head_y as usize] {
            Tile::Snake(_) => {
                // Die if collided
                self.alive = false;
                return;
            }
            Tile::Food => {
                // Eat
                self.length += 1;
                self.spawn_food();
                self.tiles[self.head_x as usize][self.head_y as usize] = Tile::Snake(0);
            }
            Tile::Empty => {
                // Set head position to snake tile
                self.tiles[self.head_x as usize][self.head_y as usize] = Tile::Snake(0);
            }
        }

        // Update the grid's snake values
        for x in 0..self.width {
            for y in 0..self.height {
                match self.tiles[x as usize][y as usize] {
                    Tile::Snake(val) => {
                        self.tiles[x as usize][y as usize] = if val >= self.length {
                            Tile::Empty
                        } else {
                            Tile::Snake(val + 1)
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    fn spawn_food(&mut self) {
        loop {
            let tile = &mut self.tiles[thread_rng().gen_range(0, self.width) as usize]
            [thread_rng().gen_range(0, self.height) as usize];
            if *tile == Tile::Empty {
                *tile = Tile::Food;  
                break;
            } 
        }
    }

    fn render_ascii(&self) {
        // Top border
        stdout().write("  ".as_bytes()).unwrap();
        for _x in 0..self.width {
            stdout().write("--".as_bytes()).unwrap();
        }
        stdout().write("\n".as_bytes()).unwrap();

        for y in 0..self.height {
            // Left border
            stdout().write("| ".as_bytes()).unwrap();

            // Tiles
            for x in 0..self.width {
                stdout()
                    .write(self.tiles[x as usize][y as usize].ascii_rep().as_bytes())
                    .unwrap();
            }

            // Right border
            stdout().write(" |\n".as_bytes()).unwrap();
        }

        // Bottom border
        stdout().write("  ".as_bytes()).unwrap();
        for _x in 0..self.width {
            stdout().write("--".as_bytes()).unwrap();
        }
        stdout().write("\n".as_bytes()).unwrap();
    }
}

// Snake direction controls
#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    // Get the opposite direction
    fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

// Possible states of a tile
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tile {
    Empty,
    Food,
    Snake(SnakeVal),
}

impl Tile {
    // Get a two-character ASCII representation
    fn ascii_rep(self) -> &'static str {
        match self {
            Tile::Empty => "  ",
            Tile::Food => "><",
            Tile::Snake(_) => "██",
        }
    }
}
