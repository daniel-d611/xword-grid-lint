// Randomized grid generation: place blocks in symmetric pairs at random and
// keep the result only if it already passes the checks in grid.rs
// (connectivity, minimum word length, block density). This is generate-and-
// test rather than a constructive algorithm, so it can fail on tight
// constraints (very short minimum word length combined with very low block
// density on a small grid); GenerateOptions::max_attempts controls how hard
// it tries before giving up.
//
// Only 180-degree rotational symmetry is supported, since that's the only
// symmetry grid.rs knows how to check. A grid built under a different
// symmetry would just get reported as broken by the rest of the tool.

use crate::grid::Grid;

pub struct GenerateOptions {
    pub width: usize,
    pub height: usize,
    pub max_block_density: f64,
    pub min_word_length: usize,
    pub seed: u64,
}

const DEFAULT_MAX_ATTEMPTS: usize = 4000;

pub fn generate(opts: &GenerateOptions) -> Result<Grid, String> {
    if opts.width == 0 || opts.height == 0 {
        return Err("grid dimensions must be at least 1x1".to_string());
    }

    let mut rng = Rng::new(opts.seed);
    for _ in 0..DEFAULT_MAX_ATTEMPTS {
        if let Some(grid) = try_generate(opts, &mut rng) {
            return Ok(grid);
        }
    }
    Err(format!(
        "couldn't find a valid {}x{} grid satisfying the constraints in {} attempts; \
         try a lower --min-word-length or a higher --max-block-density",
        opts.width, opts.height, DEFAULT_MAX_ATTEMPTS
    ))
}

fn try_generate(opts: &GenerateOptions, rng: &mut Rng) -> Option<Grid> {
    let width = opts.width;
    let height = opts.height;

    let mut pairs = symmetric_pairs(width, height);
    rng.shuffle(&mut pairs);

    let target_blocks = (opts.max_block_density * (width * height) as f64).floor() as usize;

    let mut blocked = vec![vec![false; width]; height];
    let mut block_count = 0;
    for (a, b) in pairs {
        if block_count >= target_blocks {
            break;
        }
        let added = if a == b { 1 } else { 2 };
        if block_count + added > target_blocks {
            continue;
        }
        blocked[a.0][a.1] = true;
        blocked[b.0][b.1] = true;
        block_count += added;
    }

    let grid = Grid::from_blocked(width, height, blocked);

    if !grid.is_connected() || !grid.isolated_cells().is_empty() {
        return None;
    }
    for entry in grid.entries() {
        if entry.across_len.is_some_and(|l| l < opts.min_word_length)
            || entry.down_len.is_some_and(|l| l < opts.min_word_length)
        {
            return None;
        }
    }

    Some(grid)
}

// Each cell paired with its 180-degree rotational mirror. A cell that maps
// to itself (the center of a grid with odd width and odd height) appears
// once as a pair of itself rather than twice.
fn symmetric_pairs(width: usize, height: usize) -> Vec<((usize, usize), (usize, usize))> {
    let mut pairs = Vec::new();
    let mut seen = vec![vec![false; width]; height];
    for r in 0..height {
        for c in 0..width {
            if seen[r][c] {
                continue;
            }
            let (mr, mc) = (height - 1 - r, width - 1 - c);
            seen[r][c] = true;
            seen[mr][mc] = true;
            pairs.push(((r, c), (mr, mc)));
        }
    }
    pairs
}

// xorshift64* - small enough to hand-roll, good enough for shuffling block
// positions. Not suitable for anything security-sensitive, but nothing here
// is.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn gen_below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.gen_below(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(width: usize, height: usize) -> GenerateOptions {
        GenerateOptions {
            width,
            height,
            max_block_density: 0.2,
            min_word_length: 3,
            seed: 12345,
        }
    }

    #[test]
    fn generated_grid_passes_the_checks_it_was_built_for() {
        let grid = generate(&opts(9, 9)).unwrap();
        assert_eq!(grid.symmetry_mismatches(), 0);
        assert!(grid.is_connected());
        assert!(grid.isolated_cells().is_empty());
        assert!(grid.block_density() <= 0.2);
        for entry in grid.entries() {
            if let Some(l) = entry.across_len {
                assert!(l >= 3, "across entry shorter than minimum: {}", l);
            }
            if let Some(l) = entry.down_len {
                assert!(l >= 3, "down entry shorter than minimum: {}", l);
            }
        }
    }

    #[test]
    fn same_seed_produces_the_same_grid() {
        let a = generate(&opts(9, 9)).unwrap();
        let b = generate(&opts(9, 9)).unwrap();
        assert_eq!(a.to_sketch(), b.to_sketch());
    }

    #[test]
    fn different_seeds_can_produce_different_grids() {
        let mut a = opts(11, 11);
        a.seed = 1;
        let mut b = opts(11, 11);
        b.seed = 2;
        let grid_a = generate(&a).unwrap();
        let grid_b = generate(&b).unwrap();
        assert_ne!(grid_a.to_sketch(), grid_b.to_sketch());
    }

    #[test]
    fn zero_sized_grid_is_rejected() {
        assert!(generate(&opts(0, 5)).is_err());
        assert!(generate(&opts(5, 0)).is_err());
    }

    #[test]
    fn generate_works_on_a_grid_with_no_blocks_allowed() {
        let mut o = opts(5, 5);
        o.max_block_density = 0.0;
        let grid = generate(&o).unwrap();
        assert_eq!(grid.block_cells(), 0);
    }

    #[test]
    fn odd_by_odd_grid_handles_the_self_symmetric_center_cell() {
        let grid = generate(&opts(7, 7)).unwrap();
        assert_eq!(grid.symmetry_mismatches(), 0);
    }
}
