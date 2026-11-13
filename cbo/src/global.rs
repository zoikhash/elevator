use crate::{Elevator, Request, Stop, CAPACITY};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Done,
    Waiting,
    Aboard(usize),
}

#[derive(Clone)]
struct Plan {
    floor: u32,
    time: u64,
    occupants: usize,
    route: Vec<Stop>,
}

pub struct Report {
    pub optimal: bool,
    pub cost: u64,
    pub initial_cost: u64,
}

struct Search<'a> {
    requests: &'a [Request],
    deadline: Instant,
    timed_out: bool,
    best_cost: u64,
    best_routes: Vec<Vec<Stop>>,
}

impl Search<'_> {
    // Ignore shared capacity and competing stops for an optimistic completion
    // bound. Only the current and not-yet-planned elevators remain available.
    fn lower_bound(&self, elevator: usize, plans: &[Plan], states: &[State], cost: u64) -> u64 {
        let mut bound = cost;
        for (i, state) in states.iter().enumerate() {
            let request = &self.requests[i];
            let completion = match *state {
                State::Done => continue,
                State::Waiting => plans[elevator..]
                    .iter()
                    .map(|plan| {
                        plan.time
                            + u64::from(plan.floor.abs_diff(request.source))
                            + u64::from(request.source.abs_diff(request.dest))
                    })
                    .min()
                    .expect("an unfinished plan has an available elevator"),
                State::Aboard(owner) => {
                    plans[owner].time + u64::from(plans[owner].floor.abs_diff(request.dest))
                }
            };
            bound += completion - request.time;
        }
        bound
    }

    fn visit(&mut self, elevator: usize, plans: &mut [Plan], states: &mut [State], cost: u64) {
        if Instant::now() >= self.deadline {
            self.timed_out = true;
            return;
        }
        if states.iter().all(|state| *state == State::Done) {
            if cost < self.best_cost {
                self.best_cost = cost;
                self.best_routes = plans.iter().map(|plan| plan.route.clone()).collect();
            }
            return;
        }
        if elevator == plans.len()
            || self.lower_bound(elevator, plans, states, cost) >= self.best_cost
        {
            return;
        }

        let mut choices = Vec::new();
        for (i, state) in states.iter().enumerate() {
            match *state {
                State::Waiting if plans[elevator].occupants < CAPACITY => {
                    choices.push(Stop::Pickup(i));
                }
                State::Aboard(owner) if owner == elevator => choices.push(Stop::Dropoff(i)),
                _ => {}
            }
        }
        choices.sort_by_key(|stop| plans[elevator].floor.abs_diff(stop.floor(self.requests)));
        for stop in choices {
            let (Stop::Pickup(i) | Stop::Dropoff(i)) = stop;
            let old_state = states[i];
            let old_floor = plans[elevator].floor;
            let old_time = plans[elevator].time;
            let old_occupants = plans[elevator].occupants;
            let floor = stop.floor(self.requests);
            let plan = &mut plans[elevator];
            if !plan.route.is_empty() && floor != plan.floor {
                plan.time += 1;
            }
            plan.time += u64::from(plan.floor.abs_diff(floor));
            plan.floor = floor;
            plan.route.push(stop);
            let mut next_cost = cost;
            match stop {
                Stop::Pickup(_) => {
                    plan.occupants += 1;
                    states[i] = State::Aboard(elevator);
                }
                Stop::Dropoff(_) => {
                    plan.occupants -= 1;
                    states[i] = State::Done;
                    next_cost += plan.time - self.requests[i].time;
                }
            }
            self.visit(elevator, plans, states, next_cost);
            let plan = &mut plans[elevator];
            plan.floor = old_floor;
            plan.time = old_time;
            plan.occupants = old_occupants;
            plan.route.pop();
            states[i] = old_state;
            if self.timed_out {
                return;
            }
        }
        // Finalize this elevator's complete route, then partition the remaining
        // waiting requests among later elevators. Onboard passengers cannot move.
        if plans[elevator].occupants == 0 {
            self.visit(elevator + 1, plans, states, cost);
        }
    }
}

pub fn optimize(
    elevators: &mut [Elevator],
    requests: &[Request],
    now: u64,
    limit: Duration,
) -> Report {
    let deadline = Instant::now() + limit;
    let initial_cost = elevators
        .iter()
        .map(|elevator| {
            crate::route_cost(elevator, &elevator.route, requests, now)
                .expect("incumbent route is feasible")
        })
        .sum();
    let mut states = vec![State::Done; requests.len()];
    for (owner, elevator) in elevators.iter().enumerate() {
        for &stop in &elevator.route {
            match stop {
                Stop::Pickup(i) => states[i] = State::Waiting,
                Stop::Dropoff(i) if states[i] != State::Waiting => {
                    states[i] = State::Aboard(owner);
                }
                _ => {}
            }
        }
    }
    let mut plans: Vec<_> = elevators
        .iter()
        .map(|elevator| Plan {
            floor: elevator.floor,
            time: now,
            occupants: elevator.occupants,
            route: Vec::new(),
        })
        .collect();
    let mut search = Search {
        requests,
        deadline,
        timed_out: false,
        best_cost: initial_cost,
        best_routes: elevators
            .iter()
            .map(|elevator| elevator.route.clone())
            .collect(),
    };
    search.visit(0, &mut plans, &mut states, 0);
    for (elevator, route) in elevators.iter_mut().zip(search.best_routes) {
        elevator.route = route;
    }
    Report {
        optimal: !search.timed_out,
        cost: search.best_cost,
        initial_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_independent_exhaustive_enumeration() {
        fn permutations(stops: &mut [Stop], index: usize, output: &mut Vec<Vec<Stop>>) {
            if index == stops.len() {
                output.push(stops.to_vec());
                return;
            }
            for i in index..stops.len() {
                stops.swap(index, i);
                permutations(stops, index + 1, output);
                stops.swap(index, i);
            }
        }
        let mut orders = Vec::new();
        permutations(
            &mut [
                Stop::Pickup(0),
                Stop::Dropoff(0),
                Stop::Pickup(1),
                Stop::Dropoff(1),
                Stop::Pickup(2),
                Stop::Dropoff(2),
            ],
            0,
            &mut orders,
        );
        for offset in 0..4 {
            let requests: Vec<_> = (0..3)
                .map(|i| Request {
                    time: i as u64,
                    source: (i * 3 + offset) % 7,
                    dest: (i * 2 + offset) % 7,
                })
                .collect();
            let mut elevators = vec![
                Elevator::default(),
                Elevator {
                    floor: 6,
                    ..Elevator::default()
                },
            ];
            let mut optimum = u64::MAX;
            for assignment in 0..8 {
                for order in &orders {
                    let mut routes = [Vec::new(), Vec::new()];
                    let mut picked = [false; 3];
                    let mut valid = true;
                    for &stop in order {
                        let (Stop::Pickup(i) | Stop::Dropoff(i)) = stop;
                        match stop {
                            Stop::Pickup(_) => picked[i] = true,
                            Stop::Dropoff(_) if !picked[i] => {
                                valid = false;
                                break;
                            }
                            _ => {}
                        }
                        routes[(assignment >> i) & 1].push(stop);
                    }
                    if valid {
                        let cost: Option<u64> = elevators
                            .iter()
                            .zip(routes)
                            .map(|(e, r)| crate::route_cost(e, &r, &requests, 3))
                            .sum();
                        if let Some(cost) = cost {
                            optimum = optimum.min(cost);
                        }
                    }
                }
            }
            for i in 0..requests.len() {
                crate::assign(i, &mut elevators, &requests, 3);
            }
            let report = optimize(&mut elevators, &requests, 3, Duration::from_secs(2));
            assert!(report.optimal);
            assert_eq!(report.cost, optimum);
            let actual: u64 = elevators
                .iter()
                .map(|e| crate::route_cost(e, &e.route, &requests, 3).unwrap())
                .sum();
            assert_eq!(actual, optimum);
        }
    }

    #[test]
    fn jointly_reassigns_and_proves_optimum() {
        let requests = vec![
            Request {
                time: 0,
                source: 0,
                dest: 2,
            },
            Request {
                time: 0,
                source: 10,
                dest: 8,
            },
        ];
        let mut elevators = vec![
            Elevator {
                floor: 10,
                occupants: 0,
                route: vec![Stop::Pickup(0), Stop::Dropoff(0)],
            },
            Elevator {
                floor: 0,
                occupants: 0,
                route: vec![Stop::Pickup(1), Stop::Dropoff(1)],
            },
        ];
        let report = optimize(&mut elevators, &requests, 0, Duration::from_secs(2));
        assert!(report.optimal);
        assert_eq!(report.cost, 6);
        assert!(report.cost < report.initial_cost);
        assert_eq!(elevators[0].route, vec![Stop::Pickup(1), Stop::Dropoff(1)]);
    }

    #[test]
    fn keeps_onboard_passenger_and_returns_incumbent_on_timeout() {
        let requests = vec![Request {
            time: 0,
            source: 0,
            dest: 3,
        }];
        let original = vec![
            Elevator {
                floor: 1,
                occupants: 1,
                route: vec![Stop::Dropoff(0)],
            },
            Elevator::default(),
        ];
        let mut elevators = original.clone();
        let timed = optimize(&mut elevators, &requests, 1, Duration::ZERO);
        assert!(!timed.optimal);
        assert_eq!(elevators[0].route, original[0].route);
        let exact = optimize(&mut elevators, &requests, 1, Duration::from_secs(2));
        assert!(exact.optimal);
        assert_eq!(exact.cost, 3);
        assert_eq!(elevators[0].route, vec![Stop::Dropoff(0)]);
        assert!(elevators[1].route.is_empty());
    }

    #[test]
    fn time_limited_plan_preserves_all_requests_and_capacity() {
        let requests: Vec<_> = (0..12)
            .map(|i| Request {
                time: 0,
                source: i % 4,
                dest: 9 - i % 4,
            })
            .collect();
        let mut elevators = vec![Elevator::default()];
        for i in 0..requests.len() {
            crate::assign(i, &mut elevators, &requests, 0);
        }
        let report = optimize(&mut elevators, &requests, 0, Duration::from_millis(5));
        assert!(report.cost <= report.initial_cost);
        let route = &elevators[0].route;
        assert_eq!(
            crate::route_cost(&elevators[0], route, &requests, 0),
            Some(report.cost)
        );
        assert_eq!(route.len(), 2 * requests.len());
        for i in 0..requests.len() {
            let pickup = route.iter().position(|s| *s == Stop::Pickup(i)).unwrap();
            let dropoff = route.iter().position(|s| *s == Stop::Dropoff(i)).unwrap();
            assert!(pickup < dropoff);
        }
    }
}
