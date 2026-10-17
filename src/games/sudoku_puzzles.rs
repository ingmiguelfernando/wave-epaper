//! Seeded Sudoku puzzle generator: a shuffled-backtracking solution grid,
//! then cell removal while exactly one solution survives.

use super::sudoku::SUDOKU_CELL_COUNT;

/// Target givens per difficulty; removal stops earlier when uniqueness does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SudokuDifficulty {
    Easy,
    Medium,
    Hard,
}

impl SudokuDifficulty {
    /// Every difficulty, easiest first, as the start list offers them.
    pub const ALL: [Self; 3] = [Self::Easy, Self::Medium, Self::Hard];

    #[must_use]
    pub const fn target_givens(self) -> usize {
        match self {
            Self::Easy => 40,
            Self::Medium => 32,
            Self::Hard => 26,
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    /// The title-bar caption, as the mockup shows it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "easy" => Some(Self::Easy),
            "medium" => Some(Self::Medium),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
}

/// Small xorshift state, the same shape `tetris.rs` uses; zero is reserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u32) as usize
    }
}

/// A generated puzzle with its full solution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedPuzzle {
    /// 81 digits, 0 for the cells to fill.
    pub puzzle: [u8; SUDOKU_CELL_COUNT],
    pub solution: [u8; SUDOKU_CELL_COUNT],
    pub difficulty: SudokuDifficulty,
    pub givens: usize,
}

/// Generate one puzzle of `difficulty` from `seed`. The same seed yields the
/// same puzzle.
#[must_use]
pub fn generate(difficulty: SudokuDifficulty, seed: u32) -> GeneratedPuzzle {
    let mut random = Xorshift(if seed == 0 { 0x9E37_79B9 } else { seed });
    let mut solution = [0_u8; SUDOKU_CELL_COUNT];
    fill_solution(&mut solution, &mut random);
    let mut puzzle = solution;
    let mut order: Vec<usize> = (0..SUDOKU_CELL_COUNT).collect();
    shuffle(&mut order, &mut random);
    let mut givens = SUDOKU_CELL_COUNT;
    for &index in &order {
        if givens <= difficulty.target_givens() {
            break;
        }
        let removed = puzzle[index];
        puzzle[index] = 0;
        if count_solutions(&puzzle, 2) != 1 {
            puzzle[index] = removed;
        } else {
            givens -= 1;
        }
    }
    GeneratedPuzzle {
        puzzle,
        solution,
        difficulty,
        givens,
    }
}

/// Fill an empty grid by backtracking with shuffled candidates. A loop, not
/// recursion: 81 nested calls overflowed the main task's stack.
fn fill_solution(grid: &mut [u8; SUDOKU_CELL_COUNT], random: &mut Xorshift) {
    // Each cell's shuffled digits and how many of them were tried.
    let mut digits = [[0_u8; 9]; SUDOKU_CELL_COUNT];
    let mut tried = [0_usize; SUDOKU_CELL_COUNT];
    let mut index = 0;
    let mut entering = true;
    while index < SUDOKU_CELL_COUNT {
        if entering {
            digits[index] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
            shuffle(&mut digits[index], random);
            tried[index] = 0;
        }
        grid[index] = 0;
        let mut placed = false;
        while tried[index] < 9 {
            let value = digits[index][tried[index]];
            tried[index] += 1;
            if !conflicts(grid, index, value) {
                grid[index] = value;
                placed = true;
                break;
            }
        }
        if placed {
            index += 1;
            entering = true;
        } else if index == 0 {
            return;
        } else {
            index -= 1;
            entering = false;
        }
    }
}

/// True when `value` already appears in `index`'s row, column or box.
fn conflicts(grid: &[u8; SUDOKU_CELL_COUNT], index: usize, value: u8) -> bool {
    let row = index / 9;
    let column = index % 9;
    for peer in 0..9 {
        if grid[row * 9 + peer] == value && row * 9 + peer != index {
            return true;
        }
        if grid[peer * 9 + column] == value && peer * 9 + column != index {
            return true;
        }
    }
    let box_row = (row / 3) * 3;
    let box_column = (column / 3) * 3;
    for dy in 0..3 {
        for dx in 0..3 {
            let peer = (box_row + dy) * 9 + box_column + dx;
            if peer != index && grid[peer] == value {
                return true;
            }
        }
    }
    false
}

fn shuffle<T>(items: &mut [T], random: &mut Xorshift) {
    for end in (1..items.len()).rev() {
        items.swap(random.below(end + 1), end);
    }
}

/// Bits 1 to 9 of a unit's used-value mask.
const ALL_VALUES: u16 = 0x3FE;

/// Count solutions up to `limit`; the generator keeps removals that leave 1.
fn count_solutions(grid: &[u8; SUDOKU_CELL_COUNT], limit: usize) -> usize {
    // Used values per row, column and box, one bit per digit.
    let mut used = [[0_u16; 9]; 3];
    for (index, &value) in grid.iter().enumerate() {
        if value == 0 {
            continue;
        }
        let bit = 1 << value;
        let [row, column, square] = units(index);
        if (used[0][row] | used[1][column] | used[2][square]) & bit != 0 {
            return 0;
        }
        used[0][row] |= bit;
        used[1][column] |= bit;
        used[2][square] |= bit;
    }
    let mut grid = *grid;
    count_from(&mut grid, &mut used, limit)
}

/// Row, column and box of a cell.
const fn units(index: usize) -> [usize; 3] {
    let (row, column) = (index / 9, index % 9);
    [row, column, row / 3 * 3 + column / 3]
}

/// The empty cell with the fewest candidates and those candidates; `None`
/// when the grid is full.
fn fewest_options(grid: &[u8; SUDOKU_CELL_COUNT], used: &[[u16; 9]; 3]) -> Option<(usize, u16)> {
    let mut best: Option<(usize, u16, u32)> = None;
    for (index, &value) in grid.iter().enumerate() {
        if value != 0 {
            continue;
        }
        let [row, column, square] = units(index);
        let options = !(used[0][row] | used[1][column] | used[2][square]) & ALL_VALUES;
        let count = options.count_ones();
        if best.map_or(true, |(_, _, fewest)| count < fewest) {
            best = Some((index, options, count));
            if count <= 1 {
                break;
            }
        }
    }
    best.map(|(index, options, _)| (index, options))
}

/// Fill the empty cell with the fewest candidates first. A sparse Hard grid
/// then takes thousands of steps instead of millions, so the device's
/// generator answers within a press. A loop with its own stack of cells,
/// not recursion, for the main task's stack.
fn count_from(grid: &mut [u8; SUDOKU_CELL_COUNT], used: &mut [[u16; 9]; 3], limit: usize) -> usize {
    // The cells being filled, each with the candidates still to try.
    let mut path = [(0_usize, 0_u16); SUDOKU_CELL_COUNT];
    let mut depth = 0;
    let mut found = 0;
    loop {
        match fewest_options(grid, used) {
            Some((index, options)) => {
                path[depth] = (index, options);
                depth += 1;
            }
            None => {
                found += 1;
                if found >= limit {
                    return found;
                }
            }
        }
        // The next candidate of the deepest cell that has one left.
        loop {
            let Some(&(index, options)) = depth.checked_sub(1).map(|top| &path[top]) else {
                return found;
            };
            let [row, column, square] = units(index);
            if grid[index] != 0 {
                let bit = 1 << grid[index];
                used[0][row] &= !bit;
                used[1][column] &= !bit;
                used[2][square] &= !bit;
                grid[index] = 0;
            }
            if options == 0 {
                depth -= 1;
                continue;
            }
            let bit = options & options.wrapping_neg();
            path[depth - 1].1 = options & (options - 1);
            grid[index] = bit.trailing_zeros() as u8;
            used[0][row] |= bit;
            used[1][column] |= bit;
            used[2][square] |= bit;
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        conflicts, count_solutions, generate, shuffle, units, GeneratedPuzzle, SudokuDifficulty,
        Xorshift, ALL_VALUES,
    };

    /// A tiny solver for the tests: counts solutions of a copy.
    fn solve_and_count(grid: &[u8; 81], limit: usize) -> usize {
        count_solutions(grid, limit)
    }

    /// The recursive generator of v0.12.3 and before, kept to show the loops
    /// give the same puzzles.
    fn recursive_generate(difficulty: SudokuDifficulty, seed: u32) -> GeneratedPuzzle {
        let mut random = Xorshift(if seed == 0 { 0x9E37_79B9 } else { seed });
        let mut solution = [0_u8; 81];
        recursive_fill(&mut solution, 0, &mut random);
        let mut puzzle = solution;
        let mut order: Vec<usize> = (0..81).collect();
        shuffle(&mut order, &mut random);
        let mut givens = 81;
        for &index in &order {
            if givens <= difficulty.target_givens() {
                break;
            }
            let removed = puzzle[index];
            puzzle[index] = 0;
            if recursive_count_solutions(&puzzle, 2) != 1 {
                puzzle[index] = removed;
            } else {
                givens -= 1;
            }
        }
        GeneratedPuzzle {
            puzzle,
            solution,
            difficulty,
            givens,
        }
    }

    fn recursive_fill(grid: &mut [u8; 81], index: usize, random: &mut Xorshift) -> bool {
        if index == 81 {
            return true;
        }
        let mut digits = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9];
        shuffle(&mut digits, random);
        for &value in &digits {
            if !conflicts(grid, index, value) {
                grid[index] = value;
                if recursive_fill(grid, index + 1, random) {
                    return true;
                }
                grid[index] = 0;
            }
        }
        false
    }

    fn recursive_count_solutions(grid: &[u8; 81], limit: usize) -> usize {
        let mut used = [[0_u16; 9]; 3];
        for (index, &value) in grid.iter().enumerate() {
            if value == 0 {
                continue;
            }
            let bit = 1 << value;
            let [row, column, square] = units(index);
            if (used[0][row] | used[1][column] | used[2][square]) & bit != 0 {
                return 0;
            }
            used[0][row] |= bit;
            used[1][column] |= bit;
            used[2][square] |= bit;
        }
        let mut grid = *grid;
        recursive_count(&mut grid, &mut used, limit)
    }

    fn recursive_count(grid: &mut [u8; 81], used: &mut [[u16; 9]; 3], limit: usize) -> usize {
        let mut best: Option<(usize, u16, u32)> = None;
        for (index, &value) in grid.iter().enumerate() {
            if value != 0 {
                continue;
            }
            let [row, column, square] = units(index);
            let options = !(used[0][row] | used[1][column] | used[2][square]) & ALL_VALUES;
            let count = options.count_ones();
            if best.map_or(true, |(_, _, fewest)| count < fewest) {
                best = Some((index, options, count));
                if count <= 1 {
                    break;
                }
            }
        }
        let Some((index, mut options, _)) = best else {
            return 1;
        };
        let [row, column, square] = units(index);
        let mut found = 0;
        while options != 0 && found < limit {
            let bit = options & options.wrapping_neg();
            options &= options - 1;
            grid[index] = bit.trailing_zeros() as u8;
            used[0][row] |= bit;
            used[1][column] |= bit;
            used[2][square] |= bit;
            found += recursive_count(grid, used, limit - found);
            used[0][row] &= !bit;
            used[1][column] &= !bit;
            used[2][square] &= !bit;
        }
        grid[index] = 0;
        found
    }

    #[test]
    fn the_loops_give_the_puzzles_the_recursion_gave() {
        for difficulty in SudokuDifficulty::ALL {
            for seed in [0, 1, 7, 42, 99, 1234, 0xDEAD_BEEF] {
                assert_eq!(
                    generate(difficulty, seed),
                    recursive_generate(difficulty, seed),
                    "{difficulty:?} seed {seed}"
                );
            }
        }
    }

    #[test]
    fn counting_stops_at_the_limit_and_finds_clashes() {
        let empty = [0_u8; 81];
        assert_eq!(count_solutions(&empty, 2), 2);
        assert_eq!(count_solutions(&empty, 5), 5);
        let mut clash = [0_u8; 81];
        clash[0] = 5;
        clash[1] = 5;
        assert_eq!(count_solutions(&clash, 2), 0);
        let full = generate(SudokuDifficulty::Easy, 7).solution;
        assert_eq!(count_solutions(&full, 2), 1);
    }

    #[test]
    fn same_seed_gives_the_same_puzzle() {
        for difficulty in [
            SudokuDifficulty::Easy,
            SudokuDifficulty::Medium,
            SudokuDifficulty::Hard,
        ] {
            let left = generate(difficulty, 7);
            let right = generate(difficulty, 7);
            assert_eq!(left, right, "{difficulty:?}");
        }
    }

    #[test]
    fn givens_hit_the_difficulty_target_or_stop_early() {
        for (difficulty, ceiling) in [
            (SudokuDifficulty::Easy, 40),
            (SudokuDifficulty::Medium, 32),
            (SudokuDifficulty::Hard, 26),
        ] {
            let puzzle = generate(difficulty, 42);
            assert!(
                puzzle.givens <= ceiling,
                "{difficulty:?}: {}",
                puzzle.givens
            );
            assert!(puzzle.givens >= 24, "{difficulty:?}: {}", puzzle.givens);
        }
    }

    #[test]
    fn generated_puzzles_have_exactly_one_solution() {
        for seed in [0, 1, 1234] {
            let puzzle = generate(SudokuDifficulty::Hard, seed);
            assert_eq!(solve_and_count(&puzzle.puzzle, 2), 1, "seed {seed}");
            assert_eq!(solve_and_count(&puzzle.solution, 2), 1);
        }
    }

    #[test]
    fn the_solution_matches_the_puzzle_givens() {
        let puzzle = generate(SudokuDifficulty::Medium, 99);
        for (index, &given) in puzzle.puzzle.iter().enumerate() {
            if given != 0 {
                assert_eq!(given, puzzle.solution[index], "cell {index}");
            }
        }
    }
}
