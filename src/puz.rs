// Reader/writer for the Across Lite .puz binary format, scoped to what this
// tool actually tracks: the block pattern and its numbering. No letter fill
// or real clue text exists anywhere in this tool, so:
//   - reading a .puz ignores whatever solution letters are present and only
//     looks at which squares are '.' (block) vs. anything else (open);
//   - writing a .puz fills every open square with '-' (Across Lite's marker
//     for "no letter yet", the same thing constructors ship in blank grid
//     templates) and uses the entry numbering as placeholder clue text,
//     since there's no word list to draw real clues from.
//
// Checksums are computed on write so the file is well-formed, but are not
// verified on read - a mismatch there would mean a corrupted or edited
// file, not a structural problem with the grid, and this tool only cares
// about the grid.

use crate::grid::Grid;

const MAGIC: &[u8] = b"ACROSS&DOWN\0";
const MASK_LOW: [u8; 4] = *b"ICHE";
const MASK_HIGH: [u8; 4] = *b"ATED";
const HEADER_LEN: usize = 0x34;

pub fn write(grid: &Grid) -> Result<Vec<u8>, String> {
    let width = grid.width;
    let height = grid.height;
    if width == 0 || height == 0 {
        return Err("grid has no cells".to_string());
    }
    if width > 255 || height > 255 {
        return Err(format!(
            "grid is {}x{}, but the .puz format only supports dimensions up to 255x255",
            width, height
        ));
    }

    let mut solution = Vec::with_capacity(width * height);
    for r in 0..height {
        for c in 0..width {
            solution.push(if grid.is_blocked(r, c) { b'.' } else { b'-' });
        }
    }
    let state = solution.clone();

    let mut clues: Vec<String> = Vec::new();
    for e in grid.entries() {
        if e.across_len.is_some() {
            clues.push(format!("{} Across", e.number));
        }
        if e.down_len.is_some() {
            clues.push(format!("{} Down", e.number));
        }
    }
    if clues.len() > u16::MAX as usize {
        return Err("grid has too many entries to fit in the .puz clue count field".to_string());
    }

    let title = "";
    let author = "";
    let copyright = "";
    let notes = "";

    let mut cib = Vec::with_capacity(8);
    cib.push(width as u8);
    cib.push(height as u8);
    cib.extend_from_slice(&(clues.len() as u16).to_le_bytes());
    cib.extend_from_slice(&1u16.to_le_bytes()); // puzzle type: normal
    cib.extend_from_slice(&0u16.to_le_bytes()); // scrambled tag: unscrambled

    let c_cib = cksum(&cib, 0);
    let c_sol = cksum(&solution, 0);
    let c_state = cksum(&state, 0);
    let c_text = text_checksum(title, author, copyright, &clues, notes, 0);

    let mut global = c_cib;
    global = cksum(&solution, global);
    global = cksum(&state, global);
    global = text_checksum(title, author, copyright, &clues, notes, global);

    let masked_low = [
        (c_cib & 0xFF) as u8 ^ MASK_LOW[0],
        (c_sol & 0xFF) as u8 ^ MASK_LOW[1],
        (c_state & 0xFF) as u8 ^ MASK_LOW[2],
        (c_text & 0xFF) as u8 ^ MASK_LOW[3],
    ];
    let masked_high = [
        ((c_cib >> 8) & 0xFF) as u8 ^ MASK_HIGH[0],
        ((c_sol >> 8) & 0xFF) as u8 ^ MASK_HIGH[1],
        ((c_state >> 8) & 0xFF) as u8 ^ MASK_HIGH[2],
        ((c_text >> 8) & 0xFF) as u8 ^ MASK_HIGH[3],
    ];

    let mut out = Vec::with_capacity(HEADER_LEN + solution.len() * 2 + 64);
    out.extend_from_slice(&global.to_le_bytes());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&c_cib.to_le_bytes());
    out.extend_from_slice(&masked_low);
    out.extend_from_slice(&masked_high);
    out.extend_from_slice(b"1.3\0");
    out.extend_from_slice(&[0u8; 2]); // reserved
    out.extend_from_slice(&0u16.to_le_bytes()); // scrambled checksum
    out.extend_from_slice(&[0u8; 12]); // reserved
    out.extend_from_slice(&cib);
    debug_assert_eq!(out.len(), HEADER_LEN);

    out.extend_from_slice(&solution);
    out.extend_from_slice(&state);
    out.push(0); // title
    out.push(0); // author
    out.push(0); // copyright
    for clue in &clues {
        out.extend_from_slice(clue.as_bytes());
        out.push(0);
    }
    out.push(0); // notes

    Ok(out)
}

pub fn read(bytes: &[u8]) -> Result<Grid, String> {
    if bytes.len() < HEADER_LEN {
        return Err("file is too short to be a .puz file".to_string());
    }
    if &bytes[0x02..0x0E] != MAGIC {
        return Err("missing ACROSS&DOWN magic header; not a .puz file".to_string());
    }

    let width = bytes[0x2C] as usize;
    let height = bytes[0x2D] as usize;
    if width == 0 || height == 0 {
        return Err("puz header declares an empty grid".to_string());
    }

    let solution_start = HEADER_LEN;
    let solution_end = solution_start + width * height;
    if bytes.len() < solution_end {
        return Err("file is truncated before the solution grid".to_string());
    }
    let solution = &bytes[solution_start..solution_end];

    let mut sketch = String::with_capacity((width + 1) * height);
    for r in 0..height {
        for c in 0..width {
            sketch.push(if solution[r * width + c] == b'.' { '#' } else { '.' });
        }
        sketch.push('\n');
    }
    Grid::parse(&sketch)
}

fn text_checksum(
    title: &str,
    author: &str,
    copyright: &str,
    clues: &[String],
    notes: &str,
    seed: u16,
) -> u16 {
    let mut c = seed;
    c = cksum_text_part(title, c);
    c = cksum_text_part(author, c);
    c = cksum_text_part(copyright, c);
    for clue in clues {
        c = cksum(clue.as_bytes(), c);
    }
    c = cksum_text_part(notes, c);
    c
}

// Empty title/author/copyright/notes fields are still written to the file
// as a lone null byte, but (per the format) don't contribute to any
// checksum at all - not even that null byte.
fn cksum_text_part(s: &str, seed: u16) -> u16 {
    if s.is_empty() {
        seed
    } else {
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0);
        cksum(&bytes, seed)
    }
}

fn cksum(data: &[u8], seed: u16) -> u16 {
    let mut c = seed;
    for &b in data {
        c = if c & 1 != 0 {
            (c >> 1).wrapping_add(0x8000)
        } else {
            c >> 1
        };
        c = c.wrapping_add(b as u16);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_block_pattern_and_numbering() {
        let grid = Grid::parse("..#..\n.....\n#...#\n.....\n..#..").unwrap();
        let bytes = write(&grid).unwrap();
        let round_tripped = read(&bytes).unwrap();

        assert_eq!(
            (grid.width, grid.height),
            (round_tripped.width, round_tripped.height)
        );
        assert_eq!(grid.entries(), round_tripped.entries());
        assert_eq!(grid.symmetry_mismatches(), round_tripped.symmetry_mismatches());
    }

    #[test]
    fn round_trip_fully_open_grid() {
        let grid = Grid::parse(".....\n.....\n.....\n.....\n.....").unwrap();
        let bytes = write(&grid).unwrap();
        let round_tripped = read(&bytes).unwrap();
        assert_eq!(grid.entries(), round_tripped.entries());
    }

    #[test]
    fn write_places_magic_and_dimensions_at_documented_offsets() {
        let grid = Grid::parse("...\n...\n...").unwrap();
        let bytes = write(&grid).unwrap();
        assert_eq!(&bytes[0x02..0x0E], MAGIC);
        assert_eq!(bytes[0x2C], 3);
        assert_eq!(bytes[0x2D], 3);
    }

    #[test]
    fn write_rejects_oversized_grid() {
        let mut sketch = String::new();
        for _ in 0..256 {
            sketch.push('.');
        }
        let grid = Grid::parse(&sketch).unwrap();
        assert!(write(&grid).is_err());
    }

    #[test]
    fn read_rejects_missing_magic() {
        let err = read(&[0u8; 60]).unwrap_err();
        assert!(err.contains("ACROSS&DOWN"), "unexpected error: {}", err);
    }

    #[test]
    fn read_rejects_truncated_header() {
        let err = read(&[0u8; 10]).unwrap_err();
        assert!(err.contains("too short"), "unexpected error: {}", err);
    }

    #[test]
    fn read_rejects_body_shorter_than_declared_grid() {
        let grid = Grid::parse("...\n...\n...").unwrap();
        let mut bytes = write(&grid).unwrap();
        bytes.truncate(HEADER_LEN + 3); // solution grid needs 9 bytes
        let err = read(&bytes).unwrap_err();
        assert!(err.contains("truncated"), "unexpected error: {}", err);
    }
}
