mod grid;

use grid::Grid;
use std::process::ExitCode;

const MIN_WORD_LENGTH: usize = 3;

struct Report {
    width: usize,
    height: usize,
    open_cells: usize,
    block_cells: usize,
    symmetric: bool,
    connected: bool,
    entries: Vec<grid::Entry>,
    errors: Vec<String>,
}

impl Report {
    fn to_human(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "grid: {}x{} ({} open, {} blocked)\n",
            self.width, self.height, self.open_cells, self.block_cells
        ));
        s.push_str(&format!("symmetric: {}\n", if self.symmetric { "yes" } else { "no" }));
        s.push_str(&format!("connected: {}\n", if self.connected { "yes" } else { "no" }));

        if self.errors.is_empty() {
            s.push_str("checks: all passed\n");
        } else {
            s.push_str(&format!(
                "checks: {} issue{} found\n",
                self.errors.len(),
                if self.errors.len() == 1 { "" } else { "s" }
            ));
            for e in &self.errors {
                s.push_str(&format!("  - {}\n", e));
            }
        }

        s.push_str("entries:\n");
        for e in &self.entries {
            let mut parts = Vec::new();
            if let Some(l) = e.across_len {
                parts.push(format!("{}-Across ({})", e.number, l));
            }
            if let Some(l) = e.down_len {
                parts.push(format!("{}-Down ({})", e.number, l));
            }
            s.push_str(&format!("  {}\n", parts.join(", ")));
        }
        s
    }

    fn to_json(&self) -> String {
        let entries_json: Vec<String> = self
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{{\"number\":{},\"row\":{},\"col\":{},\"across_len\":{},\"down_len\":{}}}",
                    e.number,
                    e.row,
                    e.col,
                    opt_num(e.across_len),
                    opt_num(e.down_len)
                )
            })
            .collect();

        let errors_json: Vec<String> = self
            .errors
            .iter()
            .map(|e| format!("\"{}\"", json_escape(e)))
            .collect();

        format!(
            "{{\"width\":{},\"height\":{},\"open_cells\":{},\"block_cells\":{},\"symmetric\":{},\"connected\":{},\"errors\":[{}],\"entries\":[{}]}}",
            self.width,
            self.height,
            self.open_cells,
            self.block_cells,
            self.symmetric,
            self.connected,
            errors_json.join(","),
            entries_json.join(",")
        )
    }
}

fn opt_num(o: Option<usize>) -> String {
    match o {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn build_report(grid: &Grid) -> Report {
    let entries = grid.entries();
    let mut errors = Vec::new();

    let mismatches = grid.symmetry_mismatches();
    if mismatches > 0 {
        errors.push(format!(
            "grid is not 180-degree rotationally symmetric ({} mismatched cell pair{})",
            mismatches,
            if mismatches == 1 { "" } else { "s" }
        ));
    }

    let connected = grid.is_connected();
    if !connected {
        errors.push("open cells are not all connected".to_string());
    }

    for (r, c) in grid.isolated_cells() {
        errors.push(format!(
            "cell at row {} col {} is not part of any word",
            r + 1,
            c + 1
        ));
    }

    for e in &entries {
        if let Some(len) = e.across_len {
            if len < MIN_WORD_LENGTH {
                errors.push(format!(
                    "{}-Across is {} letters (minimum {})",
                    e.number, len, MIN_WORD_LENGTH
                ));
            }
        }
        if let Some(len) = e.down_len {
            if len < MIN_WORD_LENGTH {
                errors.push(format!(
                    "{}-Down is {} letters (minimum {})",
                    e.number, len, MIN_WORD_LENGTH
                ));
            }
        }
    }

    Report {
        width: grid.width,
        height: grid.height,
        open_cells: grid.open_cells(),
        block_cells: grid.block_cells(),
        symmetric: mismatches == 0,
        connected,
        entries,
        errors,
    }
}

fn print_help() {
    eprintln!("xword-grid-lint - check a crossword grid layout and number its entries");
    eprintln!();
    eprintln!("usage: xword-grid-lint <grid-file> [--json]");
    eprintln!();
    eprintln!("grid file format: one line per row, '#' for a blocked square,");
    eprintln!("any other character (conventionally '.') for an open square.");
    eprintln!("all rows must have the same number of columns.");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json = false;
    let mut path: Option<String> = None;

    for a in &args {
        match a.as_str() {
            "--json" => json = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                if path.is_some() {
                    eprintln!("unexpected argument: {}", other);
                    return ExitCode::from(2);
                }
                path = Some(other.to_string());
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            print_help();
            return ExitCode::from(2);
        }
    };

    let input = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {}: {}", path, e);
            return ExitCode::FAILURE;
        }
    };

    let grid = match Grid::parse(&input) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error parsing grid: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let report = build_report(&grid);
    if json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.to_human());
    }

    if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
