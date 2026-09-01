// Grid layout is block-only: '#' means a blocked square, anything else
// (conventionally '.') means an open square. Letters aren't tracked here
// because the checks in this tool are about structure, not fill.

pub struct Grid {
    pub width: usize,
    pub height: usize,
    blocked: Vec<Vec<bool>>,
}

pub struct Entry {
    pub number: u32,
    pub row: usize,
    pub col: usize,
    pub across_len: Option<usize>,
    pub down_len: Option<usize>,
}

impl Grid {
    pub fn parse(input: &str) -> Result<Self, String> {
        let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() {
            return Err("grid is empty".to_string());
        }

        let width = lines[0].chars().count();
        let mut blocked = Vec::with_capacity(lines.len());
        for (i, line) in lines.iter().enumerate() {
            let w = line.chars().count();
            if w != width {
                return Err(format!(
                    "row {} has {} columns, expected {} (from row 1)",
                    i + 1,
                    w,
                    width
                ));
            }
            blocked.push(line.chars().map(|ch| ch == '#').collect());
        }

        Ok(Grid {
            width,
            height: blocked.len(),
            blocked,
        })
    }

    pub fn is_blocked(&self, r: usize, c: usize) -> bool {
        self.blocked[r][c]
    }

    pub fn open_cells(&self) -> usize {
        self.blocked.iter().flatten().filter(|b| !**b).count()
    }

    pub fn block_cells(&self) -> usize {
        self.width * self.height - self.open_cells()
    }

    // For every open cell, the length of the contiguous run of open cells
    // in its row that it belongs to (not just runs that start at that cell).
    pub fn across_run_lengths(&self) -> Vec<Vec<usize>> {
        let mut lens = vec![vec![0usize; self.width]; self.height];
        for r in 0..self.height {
            let mut c = 0;
            while c < self.width {
                if self.blocked[r][c] {
                    c += 1;
                    continue;
                }
                let start = c;
                while c < self.width && !self.blocked[r][c] {
                    c += 1;
                }
                let len = c - start;
                for cc in start..c {
                    lens[r][cc] = len;
                }
            }
        }
        lens
    }

    pub fn down_run_lengths(&self) -> Vec<Vec<usize>> {
        let mut lens = vec![vec![0usize; self.width]; self.height];
        for c in 0..self.width {
            let mut r = 0;
            while r < self.height {
                if self.blocked[r][c] {
                    r += 1;
                    continue;
                }
                let start = r;
                while r < self.height && !self.blocked[r][c] {
                    r += 1;
                }
                let len = r - start;
                for rr in start..r {
                    lens[rr][c] = len;
                }
            }
        }
        lens
    }

    // Standard crossword numbering: a cell gets a number if it starts an
    // across entry, a down entry, or both. A run of length 1 doesn't count
    // as an entry.
    pub fn entries(&self) -> Vec<Entry> {
        let across = self.across_run_lengths();
        let down = self.down_run_lengths();
        let mut out = Vec::new();
        let mut number = 0u32;

        for r in 0..self.height {
            for c in 0..self.width {
                if self.blocked[r][c] {
                    continue;
                }
                let starts_across = (c == 0 || self.blocked[r][c - 1]) && across[r][c] >= 2;
                let starts_down = (r == 0 || self.blocked[r - 1][c]) && down[r][c] >= 2;
                if starts_across || starts_down {
                    number += 1;
                    out.push(Entry {
                        number,
                        row: r,
                        col: c,
                        across_len: if starts_across { Some(across[r][c]) } else { None },
                        down_len: if starts_down { Some(down[r][c]) } else { None },
                    });
                }
            }
        }
        out
    }

    // Counts mismatched cell pairs under 180-degree rotation. Zero means
    // the grid has standard crossword symmetry.
    pub fn symmetry_mismatches(&self) -> usize {
        let mut mismatched = 0;
        for r in 0..self.height {
            for c in 0..self.width {
                let (mr, mc) = (self.height - 1 - r, self.width - 1 - c);
                if self.blocked[r][c] != self.blocked[mr][mc] {
                    mismatched += 1;
                }
            }
        }
        mismatched / 2
    }

    pub fn is_connected(&self) -> bool {
        let total_open = self.open_cells();
        if total_open == 0 {
            return true;
        }

        let mut start = None;
        'search: for r in 0..self.height {
            for c in 0..self.width {
                if !self.blocked[r][c] {
                    start = Some((r, c));
                    break 'search;
                }
            }
        }
        let (sr, sc) = start.unwrap();

        let mut visited = vec![vec![false; self.width]; self.height];
        let mut stack = vec![(sr, sc)];
        visited[sr][sc] = true;
        let mut count = 1;

        while let Some((r, c)) = stack.pop() {
            for (dr, dc) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr < 0 || nc < 0 || nr >= self.height as isize || nc >= self.width as isize {
                    continue;
                }
                let (nr, nc) = (nr as usize, nc as usize);
                if !self.blocked[nr][nc] && !visited[nr][nc] {
                    visited[nr][nc] = true;
                    count += 1;
                    stack.push((nr, nc));
                }
            }
        }

        count == total_open
    }

    // Open cells that belong to no across or down entry at all: a run of
    // length 1 in both directions. These can't be filled as part of any word.
    pub fn isolated_cells(&self) -> Vec<(usize, usize)> {
        let across = self.across_run_lengths();
        let down = self.down_run_lengths();
        let mut out = Vec::new();
        for r in 0..self.height {
            for c in 0..self.width {
                if self.blocked[r][c] {
                    continue;
                }
                if across[r][c] < 2 && down[r][c] < 2 {
                    out.push((r, c));
                }
            }
        }
        out
    }
}
