#![feature(universal_impl_trait)]

use std::io::prelude::*;

fn read_all<S: AsRef<std::path::Path>>(path: S) -> String {
    let mut content = String::new();
    let mut f = std::fs::File::open(path).unwrap();
    f.read_to_string(&mut content).unwrap();
    content
}

fn main() {
    let steps = std::env::args().nth(1).unwrap_or(String::from("10000")).parse().unwrap();
    let fname = std::env::args().nth(2).unwrap_or(String::from("example"));
    let content = read_all(fname);

    let cluster = ComputingCluster::new(
        Grid::from(&content), Default::default(), CurrierRule::new()
    );

    let result = infections(cluster, steps);

    println!("infections = {}", result);

    let cluster = ComputingCluster::new(
        Grid::from(&content), Default::default(), EvolvedRule::new()
    );

    let result = infections(cluster, steps);

    println!("Evolved infections = {}", result);
}

type Position = (i32, i32);

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum CellState {
    Clean,
    Weakened,
    Infected,
    Flagged,
}

use CellState::*;

#[derive(Debug)]
struct Grid(std::collections::HashMap<Position, CellState>);

impl<S: AsRef<str>> From<S> for Grid {
    fn from(data: S) -> Self {
        let lines = data.as_ref().lines().collect::<Vec<_>>();
        let h = lines.len() as i32;
        let w = lines[0].len() as i32;

        Grid(
            lines.iter().enumerate().flat_map(|(r, &l)|
                l.chars().enumerate()
                    .filter_map(move |(c, cell)|
                        if cell == '#' {Some(
                            ((r as i32 - h / 2, c as i32 - w / 2), Infected)
                        )
                        } else { None })
            ).collect()
        )
    }
}

impl Grid {
    fn state(&self, pos: Position) -> CellState {
        self.0.get(&pos).cloned().unwrap_or(Clean)
    }

    fn clean(&mut self, pos: Position) {
        self.0.remove(&pos);
    }

    fn weak(&mut self, pos: Position) {
        self.0.insert(pos, Weakened);
    }

    fn infect(&mut self, pos: Position) {
        self.0.insert(pos, Infected);
    }

    fn flag(&mut self, pos: Position) {
        self.0.insert(pos, Flagged);
    }

    fn reverse(&mut self, pos: Position) -> CellState{
        match self.state(pos) {
            Clean => {self.infect(pos); Clean},
            Infected => {self.clean(pos); Infected},
            s => s
        }
    }
}


enum Direction {
    Up,
    Right,
    Left,
    Down,
}

use Direction::*;

struct Currier {
    position: Position,
    direction: Direction,
}

impl Default for Currier {
    fn default() -> Self {
        Self::new(Default::default(), Up)
    }
}

impl Currier {
    fn new(start: Position, direction: Direction) -> Self {
        Self { position: start, direction }
    }

    fn step(&mut self) -> Position {
        match self.direction {
            Up => self.position.0 -= 1,
            Right => self.position.1 += 1,
            Left => self.position.1 -= 1,
            Down => self.position.0 += 1,
        }

        self.position
    }

    fn right(&mut self) {
        match self.direction {
            Up => self.direction = Right,
            Right => self.direction = Down,
            Left => self.direction = Up,
            Down => self.direction = Left,
        }
    }

    fn left(&mut self) {
        match self.direction {
            Up => self.direction = Left,
            Right => self.direction = Up,
            Left => self.direction = Down,
            Down => self.direction = Right,
        }
    }
}

trait Policy {
    fn apply(&self, grid: &mut Grid, currier: &mut Currier);
}

struct CurrierRule {}

impl CurrierRule {
    fn new() -> Self { Self {} }
}

impl Policy for CurrierRule {
    fn apply(&self, grid: &mut Grid, currier: &mut Currier) {
        let old_state = grid.reverse(currier.position);
        match old_state {
            Clean => currier.left(),
            Infected => currier.right(),
            _ => panic!("Not implemented : this policy cannot work with this kind of states")
        }
        currier.step();
    }
}

struct EvolvedRule {}

impl EvolvedRule {
    fn new() -> Self { Self {} }
}

impl Policy for EvolvedRule {
    fn apply(&self, grid: &mut Grid, currier: &mut Currier) {
        let old_state = grid.state(currier.position);
        let position = currier.position;
        match old_state {
            Clean => {currier.left(); grid.weak(position)},
            Infected => {currier.right(); grid.flag(position)},
            Weakened => {grid.infect(position)},
            Flagged => {currier.right();currier.right(); grid.clean(position)},
        };
        currier.step();
    }
}



struct ComputingCluster<P: Policy> {
    grid: Grid,
    currier: Currier,
    policy: P
}

impl<P: Policy> ComputingCluster<P> {
    fn new(grid: Grid, currier: Currier, policy: P) -> Self {
        ComputingCluster { grid, currier, policy }
    }
}

impl<P: Policy> Iterator for ComputingCluster<P> {
    type Item = (Position, CellState);

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.currier.position;
        self.policy.apply(&mut self.grid, &mut self.currier);

        Some((position, self.grid.state(position)))
    }
}

fn infections(cluster: ComputingCluster<impl Policy>, steps: usize) -> usize {
    cluster.take(steps).filter(|&(_, s)| s == Infected).count()
}

#[cfg(test)]
mod test {
    use super::*;

    static SIMPLE: &'static str = "\
                                    ..#\n\
                                    #..\n\
                                    ...\
                                    ";

    #[test]
    fn grid_cluster_query() {
        let grid = Grid::from(SIMPLE);

        assert_eq!(Clean, grid.state((0, 0)));
        assert_eq!(Infected, grid.state((0, -1)));
        assert_eq!(Infected, grid.state((-1, 1)));
        assert_eq!(Clean, grid.state((1, 1)));
        assert_eq!(Clean, grid.state((100, -32)));
    }

    #[test]
    fn grid_clean() {
        let mut grid = Grid::from(SIMPLE);

        grid.clean((0, -1));

        assert_eq!(Clean, grid.state((0, 1)));

        grid.clean((0, 0));

        assert_eq!(Clean, grid.state((0, 0)))
    }

    #[test]
    fn grid_infect() {
        let mut grid = Grid::from(SIMPLE);

        grid.infect((0, 0));

        assert_eq!(Infected, grid.state((0, 0)));

        grid.infect((0, -1));

        assert_eq!(Infected, grid.state((0, -1)))
    }

    #[test]
    fn grid_reverse() {
        let mut grid = Grid::from(SIMPLE);

        assert_eq!(Clean, grid.reverse((0, 0)));

        assert_eq!(Infected, grid.state((0, 0)));

        assert_eq!(Infected, grid.reverse((0, 0)));

        assert_eq!(Clean, grid.state((0, 0)));
    }


    #[test]
    fn currier_moves() {
        let mut currier = Currier::new((0, 0), Up);

        assert_eq!((-1, 0), currier.step());

        currier.right();

        assert_eq!((-1, 1), currier.step());

        currier.left();

        assert_eq!((-2, 1), currier.step());
    }

    #[test]
    fn policy_currier() {
        let grid = Grid::from(SIMPLE);
        let currier = Currier::default();

        let policy = CurrierRule {};

        let mut cluster = ComputingCluster::new(grid, currier, policy);

        assert_eq!(((0, 0), Infected), cluster.next().unwrap());
        assert_eq!(((0, -1), Clean), cluster.next().unwrap());
        assert_eq!(((-1, -1), Infected), cluster.next().unwrap());
        assert_eq!(((-1, -2), Infected), cluster.next().unwrap());
        assert_eq!(((0, -2), Infected), cluster.next().unwrap());
        assert_eq!(((0, -1), Infected), cluster.next().unwrap());
        assert_eq!(((-1, -1), Clean), cluster.next().unwrap());
    }

    #[test]
    fn count_infections() {
        let cluster = ComputingCluster::new(
            Grid::from(SIMPLE), Default::default(), CurrierRule::new()
        );

        assert_eq!(41, infections(cluster, 70))

    }

    #[test]
    fn count_lot_of_infections() {
        let cluster = ComputingCluster::new(
            Grid::from(SIMPLE), Default::default(), CurrierRule::new()
        );

        assert_eq!(5587, infections(cluster, 10000))

    }

    #[test]
    fn evolved_policy() {
        let mut grid = Grid::from(SIMPLE);
        let mut currier = Currier::default();

        let policy = EvolvedRule::new();

        // Clean
        policy.apply(&mut grid, &mut currier);

        assert_eq!(Weakened, grid.state((0,0)));
        assert_eq!((0, -1), currier.position);

        let mut currier = Currier::default();

        // Weakened
        policy.apply(&mut grid, &mut currier);

        assert_eq!(Infected, grid.state((0,0)));
        assert_eq!((-1, 0), currier.position);

        let mut currier = Currier::default();

        // Infected
        policy.apply(&mut grid, &mut currier);

        assert_eq!(Flagged, grid.state((0,0)));
        assert_eq!((0, 1), currier.position);

        let mut currier = Currier::default();

        // Flagged
        policy.apply(&mut grid, &mut currier);

        assert_eq!(Clean, grid.state((0,0)));
        assert_eq!((1, 0), currier.position);
    }

    #[test]
    fn count_infections_evolved_virus() {
        let cluster = ComputingCluster::new(
            Grid::from(SIMPLE), Default::default(), EvolvedRule::new()
        );

        assert_eq!(26, infections(cluster, 100))

    }

    #[test]
    fn count_infections_evolved_virus_10000000() {
        let cluster = ComputingCluster::new(
            Grid::from(SIMPLE), Default::default(), EvolvedRule::new()
        );

        assert_eq!(2511944, infections(cluster, 10000000))

    }
}
