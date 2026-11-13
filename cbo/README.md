## Constraint-based optimiser


Just like the Python program:

input is input.csv

output should be cbo/out/elevator_positions_log.csv

```bash
# see: main.rs
const N_FLOORS: u32 = 100;
const N_PASSENGERS: u32 = 1000;
const CAPACITY: usize = 8;
const N_ELEVATORS: usize = 6;
```

By default, it uses a heuristic, not a globally optimal solver. Add `-- --global` to jointly optimise currently known requests using branch-and-bound, with a (default) 60-second limit per arrival-epoch solve and explicit optimality status. It never considers future arrivals (look-ahead).

This program reads `input.csv` and stores an optimised elevator position route log in `cbo/out/elevator_positions_log.csv`. 


## Quickstart

```bash
cd cbo
cargo run --release --manifest-path Cargo.toml -- --global --solver-seconds 5
```


