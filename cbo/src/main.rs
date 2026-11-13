use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod global;

const N_FLOORS: u32 = 100;
const N_PASSENGERS: u32 = 1000;
const CAPACITY: usize = 8;
const N_ELEVATORS: usize = 6;

#[derive(Clone, Debug)]
struct Request {
    time: u64,
    source: u32,
    dest: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Pickup(usize),
    Dropoff(usize),
}

impl Stop {
    fn floor(self, requests: &[Request]) -> u32 {
        match self {
            Self::Pickup(i) => requests[i].source,
            Self::Dropoff(i) => requests[i].dest,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Elevator {
    floor: u32,
    occupants: usize,
    route: Vec<Stop>,
}

// Project the same movement and stop timing used by execution, rejecting
// insertions that violate capacity anywhere along the route.
fn route_cost(elevator: &Elevator, route: &[Stop], requests: &[Request], now: u64) -> Option<u64> {
    let mut floor = elevator.floor;
    let mut occupants = elevator.occupants;
    let mut time = now;
    let mut cost = 0;
    let mut previous_stop = None;
    for &stop in route {
        let target = stop.floor(requests);
        if previous_stop.is_some() && target != floor {
            time += 1;
        }
        time += u64::from(floor.abs_diff(target));
        floor = target;
        match stop {
            Stop::Pickup(_) => {
                occupants += 1;
                if occupants > CAPACITY {
                    return None;
                }
            }
            Stop::Dropoff(i) => {
                occupants = occupants.checked_sub(1)?;
                cost += time - requests[i].time;
            }
        }
        previous_stop = Some(stop);
    }
    Some(cost)
}

fn assign(request: usize, elevators: &mut [Elevator], requests: &[Request], now: u64) {
    let mut best: Option<(u64, usize, Vec<Stop>)> = None;
    for (i, elevator) in elevators.iter().enumerate() {
        let old_cost = route_cost(elevator, &elevator.route, requests, now)
            .expect("existing route must remain feasible");
        for pickup in 0..=elevator.route.len() {
            let mut with_pickup = elevator.route.clone();
            with_pickup.insert(pickup, Stop::Pickup(request));
            for dropoff in pickup + 1..=with_pickup.len() {
                let mut route = with_pickup.clone();
                route.insert(dropoff, Stop::Dropoff(request));
                if let Some(cost) = route_cost(elevator, &route, requests, now) {
                    let increase = cost - old_cost;
                    if best.as_ref().is_none_or(|(score, _, _)| increase < *score) {
                        best = Some((increase, i, route));
                    }
                }
            }
        }
    }
    let (_, elevator, route) = best.expect("appending a trip to an existing route is feasible");
    elevators[elevator].route = route;
}

#[derive(Debug)]
struct Simulation {
    positions: Vec<[u32; N_ELEVATORS]>,
    completed: usize,
    total_trip_time: u64,
    total_wait_time: u64,
}

fn simulate(requests: &[Request], global_limit: Option<Duration>) -> Simulation {
    let mut elevators: Vec<Elevator> = vec![Elevator::default(); N_ELEVATORS];
    let mut result = Simulation {
        positions: Vec::new(),
        completed: 0,
        total_trip_time: 0,
        total_wait_time: 0,
    };
    let mut pickup_times = vec![None; requests.len()];
    let mut next = 0;
    let end = requests.last().expect("input contains requests").time;
    for time in 0..=end {
        result
            .positions
            .push(std::array::from_fn(|i| elevators[i].floor));
        let previous_next = next;
        while next < requests.len() && requests[next].time == time {
            assign(next, &mut elevators, requests, time);
            next += 1;
        }
        if next != previous_next {
            if let Some(limit) = global_limit {
                let report = global::optimize(&mut elevators, &requests[..next], time, limit);
                eprintln!(
                    "epoch {time}: global {}, projected total trip time {} (insertion incumbent {})",
                    if report.optimal { "OPTIMAL" } else { "TIME LIMIT; optimality not proved" },
                    report.cost,
                    report.initial_cost
                );
            }
        }
        for elevator in &mut elevators {
            let mut serviced = 0;
            for &stop in &elevator.route {
                if stop.floor(requests) != elevator.floor {
                    break;
                }
                match stop {
                    Stop::Pickup(i) => {
                        elevator.occupants += 1;
                        pickup_times[i] = Some(time);
                    }
                    Stop::Dropoff(i) => {
                        elevator.occupants -= 1;
                        result.completed += 1;
                        result.total_trip_time += time - requests[i].time;
                        result.total_wait_time += pickup_times[i]
                            .expect("completed request must have boarded")
                            - requests[i].time;
                    }
                }
                serviced += 1;
            }
            if serviced > 0 {
                elevator.route.drain(..serviced);
            } else if let Some(stop) = elevator.route.first() {
                let target = stop.floor(requests);
                if target > elevator.floor {
                    elevator.floor += 1;
                } else {
                    elevator.floor -= 1;
                }
            }
            assert!(elevator.occupants <= CAPACITY);
        }
    }
    result
}

fn read_requests(path: &Path) -> Result<Vec<Request>, Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(path)?;
    if reader.headers()?.iter().collect::<Vec<_>>() != ["time", "passenger_id", "source", "dest"] {
        return Err("expected CSV header: time,passenger_id,source,dest".into());
    }
    let mut requests = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let parse = |column: usize| -> Result<u32, Box<dyn Error>> {
            record[column]
                .parse::<u32>()
                .map_err(|e| format!("CSV row {} column {}: {e}", i + 2, column + 1).into())
        };
        let time = parse(0)?;
        let passenger = record[1]
            .strip_prefix("passenger")
            .and_then(|id| id.parse::<u32>().ok())
            .filter(|id| (1..=N_PASSENGERS).contains(id))
            .ok_or_else(|| format!("CSV row {}: invalid passenger ID {:?}", i + 2, &record[1]))?;
        if record[1] != format!("passenger{passenger}") {
            return Err(format!("CSV row {}: noncanonical passenger ID", i + 2).into());
        }
        let source = parse(2)?;
        let dest = parse(3)?;
        if source >= N_FLOORS || dest >= N_FLOORS {
            return Err(format!("CSV row {}: floors must be in 0..{N_FLOORS}", i + 2).into());
        }
        requests.push(Request {
            time: u64::from(time),
            source,
            dest,
        });
    }
    if requests.is_empty() {
        return Err("input contains no requests".into());
    }
    requests.sort_by_key(|r| r.time);
    Ok(requests)
}

fn write_positions(path: &Path, positions: &[[u32; N_ELEVATORS]]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record((0..N_ELEVATORS).map(|i| format!("elevator_{i}")))?;
    for row in positions {
        writer.write_record(row.iter().map(u32::to_string))?;
    }
    writer.flush()?;
    Ok(())
}

struct Options {
    paths: Vec<PathBuf>,
    global_limit: Option<Duration>,
}

fn options(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Options, Box<dyn Error>> {
    let mut global = false;
    let mut seconds = 60;
    let mut limit_specified = false;
    let mut paths = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--global" {
            global = true;
        } else if arg == "--solver-seconds" {
            limit_specified = true;
            seconds = args
                .next()
                .and_then(|value| value.to_str().and_then(|s| s.parse::<u64>().ok()))
                .filter(|value| (1..=86400).contains(value))
                .ok_or("--solver-seconds requires an integer from 1 to 86400")?;
        } else if arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option: {}", arg.to_string_lossy()).into());
        } else {
            paths.push(PathBuf::from(arg));
        }
    }
    if paths.len() > 2 {
        return Err(
            "usage: elevator-cbo [--global] [--solver-seconds N] [input.csv] [output.csv]".into(),
        );
    }
    if !global && limit_specified {
        return Err("--solver-seconds requires --global".into());
    }
    Ok(Options {
        paths,
        global_limit: global.then(|| Duration::from_secs(seconds)),
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has repository parent");
    let options = options(std::env::args_os().skip(1))?;
    let input = options
        .paths
        .first()
        .cloned()
        .unwrap_or(root.join("input.csv"));
    let output = options
        .paths
        .get(1)
        .cloned()
        .unwrap_or(root.join("cbo/out/elevator_positions_log.csv"));
    if output.exists() && input.canonicalize()? == output.canonicalize()? {
        return Err("input and output paths must differ".into());
    }
    let requests = read_requests(&input)?;
    let result = simulate(&requests, options.global_limit);
    write_positions(&output, &result.positions)?;
    eprintln!(
        "Wrote {} epochs to {}. Completed {}/{} requests; {} unfinished at cutoff.",
        result.positions.len(),
        output.display(),
        result.completed,
        requests.len(),
        requests.len() - result.completed
    );
    if result.completed > 0 {
        eprintln!(
            "Mean request-to-arrival time for completed trips: {:.2} epochs.",
            result.total_trip_time as f64 / result.completed as f64
        );
        eprintln!(
            "Mean wait time for completed trips (request to boarding, excluding ride): {:.2} epochs.",
            result.total_wait_time as f64 / result.completed as f64
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(time: u64, source: u32, dest: u32) -> Request {
        Request { time, source, dest }
    }

    #[test]
    fn projection_enforces_capacity_and_dwell() {
        let requests = vec![request(0, 0, 2)];
        let elevator = Elevator::default();
        assert_eq!(
            route_cost(
                &elevator,
                &[Stop::Pickup(0), Stop::Dropoff(0)],
                &requests,
                0
            ),
            Some(3)
        );
        let full = Elevator {
            occupants: CAPACITY,
            ..elevator
        };
        assert_eq!(route_cost(&full, &[Stop::Pickup(0)], &requests, 0), None);
    }

    #[test]
    fn horizon_and_movement_match_notebook_log() {
        let result = simulate(&[request(0, 0, 2), request(6, 99, 0)], None);
        assert_eq!(result.positions.len(), 7);
        assert_eq!(result.positions[0], [0; N_ELEVATORS]);
        assert_eq!(result.positions[1][0], 0);
        assert_eq!(result.positions[2][0], 1);
        assert_eq!(result.positions[3][0], 2);
        assert_eq!(result.completed, 1);
        assert_eq!(result.total_trip_time, 3);
    }

    #[test]
    fn overloaded_pickups_still_have_feasible_routes() {
        let mut requests: Vec<_> = (0..60).map(|_| request(0, 0, 3)).collect();
        requests.push(request(50, 0, 0));
        let result = simulate(&requests, None);
        assert_eq!(result.completed, 60);
        for pair in result.positions.windows(2) {
            for (&before, &after) in pair[0].iter().zip(&pair[1]) {
                assert!(before.abs_diff(after) <= 1);
                assert!(after < N_FLOORS);
            }
        }
    }

    #[test]
    fn global_mode_does_not_anticipate_future_requests() {
        let mut requests = vec![request(0, 0, 2), request(6, 99, 0)];
        let first = simulate(&requests, Some(Duration::from_secs(1)));
        requests[1] = request(6, 0, 3);
        let second = simulate(&requests, Some(Duration::from_secs(1)));
        assert_eq!(first.positions, second.positions);
        assert_eq!(first.completed, second.completed);
        assert_eq!(first.total_trip_time, second.total_trip_time);
    }

    #[test]
    fn global_cli_options_are_validated() {
        let parse = |args: &[&str]| options(args.iter().map(std::ffi::OsString::from));
        assert_eq!(
            parse(&["--global"]).unwrap().global_limit,
            Some(Duration::from_secs(60))
        );
        assert!(parse(&["input.csv"]).unwrap().global_limit.is_none());
        assert_eq!(
            parse(&["--solver-seconds", "2", "--global"])
                .unwrap()
                .global_limit,
            Some(Duration::from_secs(2))
        );
        for args in [
            vec!["--solver-seconds", "60"],
            vec!["--global", "--solver-seconds", "0"],
            vec!["--global", "--solver-seconds", "bad"],
            vec!["--global", "--solver-seconds"],
            vec!["--typo"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn wait_time_excludes_ride_and_unfinished_trips() {
        for limit in [None, Some(Duration::from_secs(1))] {
            let result = simulate(&[request(0, 2, 4), request(10, 99, 0)], limit);
            assert_eq!(result.completed, 1);
            assert_eq!(result.total_wait_time, 2);
            assert_eq!(result.total_trip_time, 5);

            let unfinished = simulate(&[request(0, 0, 99)], limit);
            assert_eq!(unfinished.completed, 0);
            assert_eq!(unfinished.total_wait_time, 0);

            let immediate = simulate(&[request(0, 0, 2), request(6, 99, 0)], limit);
            assert_eq!(immediate.completed, 1);
            assert_eq!(immediate.total_wait_time, 0);
        }
    }
}
