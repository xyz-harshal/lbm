#[derive(Copy, Clone, Debug)]
pub enum Direction {
    Rest = 0,
    East = 1,
    North = 2,
    West = 3,
    South = 4,
    NorthEast = 5,
    NorthWest = 6,
    SouthWest = 7,
    SouthEast = 8,
}

pub fn get_direction_coordinates(i: &Direction) -> (isize, isize) {
    match i {
        Direction::Rest => (0, 0),
        Direction::East => (1, 0),
        Direction::North => (0, -1),
        Direction::West => (-1, 0),
        Direction::South => (0, 1),
        Direction::NorthEast => (1, -1),
        Direction::NorthWest => (-1, -1),
        Direction::SouthWest => (-1, 1),
        Direction::SouthEast => (1, 1),
    }
}

pub fn get_index(i: &Direction) -> usize {
    *i as usize
}

pub fn get_direction(i: usize) -> Direction {
    match i {
        0 => Direction::Rest,
        1 => Direction::East,
        2 => Direction::North,
        3 => Direction::West,
        4 => Direction::South,
        5 => Direction::NorthEast,
        6 => Direction::NorthWest,
        7 => Direction::SouthWest,
        8 => Direction::SouthEast,
        _ => Direction::Rest,
    }
}

pub fn get_weight(i: &Direction) -> f32 {
    match i {
        Direction::Rest => 4.0/9.0,
        Direction::East => 1.0/9.0,
        Direction::West => 1.0/9.0,
        Direction::North => 1.0/9.0,
        Direction::South => 1.0/9.0,
        Direction::NorthEast => 1.0/36.0,
        Direction::NorthWest => 1.0/36.0,
        Direction::SouthWest => 1.0/36.0,
        Direction::SouthEast => 1.0/36.0,
    }
}

pub fn get_opp_direction(i: &Direction) -> Direction {
    match i {
        Direction::Rest => Direction::Rest,
        Direction::East => Direction::West,
        Direction::West => Direction::East,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::NorthEast => Direction::SouthWest,
        Direction::NorthWest => Direction::SouthEast,
        Direction::SouthWest => Direction::NorthEast,
        Direction::SouthEast => Direction::NorthWest,
    }
}

struct Grid {
    //I am thinking to put the entire data in one contagious buffer for better caching and prefetching and optimization.
    //so i spans 0->8, it will give us (dx, dy) which is the direction pair.
    //we can calculate the source_cell using => i * width * height + (row * width) + col
    //Once we have located source cell we can change it's value in direction i, like:
    //target_cell = i * width * height + (target_row * width) + target_col
    //where target_row = row + dy, target_col = col + dx
    //Just have to make sure to not calculate the target_row and target_col are in bound or not
    buffer_a: Vec<f32>,
    buffer_b: Vec<f32>,
    height: usize,
    width: usize,
    tau: f32,
    u: f32,
}

impl Grid {
    pub fn new(height: usize, width: usize, tau: f32, u: f32) -> Self {
        Self {
            buffer_a: vec![0.0; height * width * 9],
            buffer_b: vec![0.0; height * width * 9],
            height,
            width,
            tau,
            u,
        }
    }

    fn index(&self, dir: &Direction, row: usize, col: usize) -> Option<usize> {
        let (dx, dy) = get_direction_coordinates(dir);
        let target_row: isize = row as isize + dy;
        let target_col: isize = col as isize + dx;
        if target_row < 0 || target_col < 0 || target_col >= self.width as isize || target_row >= self.height as isize {
            return None;
        }
        Some(get_index(dir) * self.width * self.height + (target_row as usize) * self.width + target_col as usize)
    }

    fn init(&mut self) {
        for row in 0..self.height {
            for col in 0..self.width {
                for i in 0..9 {
                    let index: usize = i * self.width * self.height + row * self.width + col;
                    self.buffer_a[index] = get_weight(&get_direction(i));
                    self.buffer_b[index] = get_weight(&get_direction(i));
                }
            }
        }
    }

    fn moments(&self, row: usize, col: usize) -> (f32, (f32, f32)){
        let mut p: f32 = 0.0;
        //loop isn't auto vectorized btw
        for i in 0..9 {
            let index: usize = i * self.width * self.height + row * self.width + col;
            p += self.buffer_a[index];
        }
        let (mut ux, mut uy): (f32, f32) = (0.0, 0.0);
        for i in 0..9 {
            let index: usize = i * self.width * self.height + row * self.width + col;
            let (dx, dy): (isize, isize) = get_direction_coordinates(&get_direction(i));
            ux += self.buffer_a[index] * (dx as f32);
            uy += self.buffer_a[index] * (dy as f32);
        }
        ux /= p;
        uy /= p;
        (p, (ux, uy))
    }

    fn equilibrium(&self, moments: (f32, (f32, f32)), dir: &Direction) -> f32 {
        let (dx, dy): (isize, isize) = get_direction_coordinates(dir);
        let (p, (ux, uy)) = moments;
        let dot: f32 = (dx as f32) * ux + (dy as f32) * uy;
        get_weight(dir) * p * (1.0 + 3.0 * dot + 4.5 * dot * dot - 1.5 * (ux * ux + uy * uy))
    }

    fn collision(&mut self, row: usize, col: usize) {
        let moments = self.moments(row, col);
        for i in 0..9 {
            let index: usize = i * self.width * self.height + row * self.width + col;
            let eq: f32 = self.equilibrium(moments, &get_direction(i));
            self.buffer_a[index] -= (1.0/self.tau) * (self.buffer_a[index] - eq);
        }
    }

    fn streaming(&mut self) {
        for row in 0..self.height {
            for col in 0..self.width {
                for i in 0..9 {
                    let source_idx: usize = i * self.width * self.height + row * self.width + col;
                    match self.index(&get_direction(i), row, col) {
                        Some(target_idx) => self.buffer_b[target_idx] = self.buffer_a[source_idx],
                        None => {
                            let j: usize = get_index(&get_opp_direction(&get_direction(i)));
                            let opposite_idx: usize = j * self.width * self.height + row * self.width + col;
                            let mut correction: f32 = 0.0;
                            if row == 0 && (i == 2 || i == 5 || i == 6) {
                                correction += 6.0 * self.moments(row, col).0 * get_weight(&get_direction(i)) * self.u * (get_direction_coordinates(&get_direction(i)).0 as f32);
                            }
                            self.buffer_b[opposite_idx] = self.buffer_a[source_idx] - correction;
                        },
                    };
                }
            }
        }
        std::mem::swap(&mut self.buffer_a, &mut self.buffer_b);
    }
}

fn main() {
    let n: usize = 129;
    let u_lid: f32 = 0.1;
    let re: f32 = 100.0;
    let nu: f32 = u_lid * ((n - 1) as f32) / re;
    let tau: f32 = 3.0 * nu + 0.5;

    let mut grid: Grid = Grid::new(n, n, tau, u_lid);
    grid.init();

    let iterations: usize = 20000;
    for _ in 0..iterations {
        for row in 0..grid.height {
            for col in 0..grid.width {
                grid.collision(row, col);
            }
        }
        grid.streaming();
    }

    // 1. Sanity check: NaN / blowup scan
    let mut max_rho_dev: f32 = 0.0;
    let mut max_speed: f32 = 0.0;
    let mut found_nan = false;

    for row in 0..grid.height {
        for col in 0..grid.width {
            let (p, (ux, uy)) = grid.moments(row, col);
            if p.is_nan() || ux.is_nan() || uy.is_nan() {
                found_nan = true;
            }
            let dev = (p - 1.0).abs();
            if dev > max_rho_dev { max_rho_dev = dev; }
            let speed = (ux * ux + uy * uy).sqrt();
            if speed > max_speed { max_speed = speed; }
        }
    }
    println!("NaN present: {}", found_nan);
    println!("Max density deviation from 1.0: {}", max_rho_dev);
    println!("Max speed: {}", max_speed);

    // 2. Centerline profile: ux along the vertical line at col = width/2
    let mid_col = grid.width / 2;
    println!("\nCenterline u-velocity (row, ux):");
    for row in 0..grid.height {
        let (_, (ux, _)) = grid.moments(row, mid_col);
        println!("{}, {}", row, ux);
    }
}
