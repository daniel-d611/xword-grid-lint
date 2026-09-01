# xword-grid-lint

A command line tool that checks a crossword grid layout against the usual
construction rules and numbers its entries the way a published puzzle would.

I build crossword grids by hand sometimes, and the tedious part isn't
picking words, it's getting the black-square pattern right: 180-degree
rotational symmetry, no word shorter than three letters, no cell that's
cut off from the rest of the grid. This tool takes a plain text sketch of
a grid and tells you what's wrong with it before you spend an evening
trying to fill it.

## Grid file format

One line per row. `#` is a blocked (black) square, any other character
is an open square, `.` by convention. All rows must have the same number
of columns.

```
.....
.....
.....
.....
.....
```

## Usage

```
xword-grid-lint <grid-file> [--json]
```

Human-readable output for a valid 5x5 grid (`examples/sample.txt`, a
fully open grid like an NYT Mini):

```
$ xword-grid-lint examples/sample.txt
grid: 5x5 (25 open, 0 blocked)
symmetric: yes
connected: yes
checks: all passed
entries:
  1-Across (5), 1-Down (5)
  2-Down (5)
  3-Down (5)
  4-Down (5)
  5-Down (5)
  6-Across (5)
  7-Across (5)
  8-Across (5)
  9-Across (5)
```

The same grid with `--json`:

```
$ xword-grid-lint examples/sample.txt --json
{"width":5,"height":5,"open_cells":25,"block_cells":0,"symmetric":true,"connected":true,"errors":[],"entries":[{"number":1,"row":0,"col":0,"across_len":5,"down_len":5},{"number":2,"row":0,"col":1,"across_len":null,"down_len":5},{"number":3,"row":0,"col":2,"across_len":null,"down_len":5},{"number":4,"row":0,"col":3,"across_len":null,"down_len":5},{"number":5,"row":0,"col":4,"across_len":null,"down_len":5},{"number":6,"row":1,"col":0,"across_len":5,"down_len":null},{"number":7,"row":2,"col":0,"across_len":5,"down_len":null},{"number":8,"row":3,"col":0,"across_len":5,"down_len":null},{"number":9,"row":4,"col":0,"across_len":5,"down_len":null}]}
```

`examples/invalid.txt` has a block pattern that isn't symmetric and two
across entries that are only two letters long, so the report includes
`errors`:

```
$ xword-grid-lint examples/invalid.txt --json
{"width":5,"height":5,"open_cells":22,"block_cells":3,"symmetric":false,"connected":true,"errors":["grid is not 180-degree rotationally symmetric (2 mismatched cell pairs)","1-Across is 2 letters (minimum 3)","3-Across is 2 letters (minimum 3)","6-Across is 2 letters (minimum 3)","7-Across is 2 letters (minimum 3)","9-Down is 2 letters (minimum 3)"],"entries":[...]}
```

The process exit code is 0 if the grid has no issues and 1 if it does,
so it can be used as a check in a build script.

## What it checks

- 180-degree rotational symmetry of the block pattern
- every open cell is reachable from every other open cell
- every open cell belongs to at least one entry (across or down, length 2+)
- every across/down entry is at least three letters long

## What it doesn't do yet

No letter fill, no word list, no grid generation. See the issue tracker
for what's planned.

## Building

Standard library only, no external crates.

```
cargo build --release
```
