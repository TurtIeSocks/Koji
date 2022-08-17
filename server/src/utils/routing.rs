use std::cmp::Ordering::Less;
use std::sync::Arc;
use vrp_pragmatic::core::models::{Problem as CoreProblem, Solution as CoreSolution};
use vrp_pragmatic::core::solver::{
    create_default_config_builder, get_default_telemetry_mode, Solver,
};
use vrp_pragmatic::core::utils::Environment;
use vrp_pragmatic::format::problem::{
    Fleet, Job, JobPlace, JobTask, Matrix, MatrixProfile, Plan, PragmaticProblem, Problem,
    ShiftStart, VehicleCosts, VehicleLimits, VehicleProfile, VehicleShift, VehicleType,
};
use vrp_pragmatic::format::solution::{create_solution, Solution};
use vrp_pragmatic::format::Location;

trait ToLocation {
    fn to_loc(self) -> Location;
}

impl ToLocation for (f64, f64) {
    fn to_loc(self) -> Location {
        let (lat, lng) = self;
        Location::new_coordinate(lat, lng)
    }
}

fn create_problem(services: Vec<[f64; 2]>, devices: Vec<String>) -> Problem {
    let tour_size = services.len() / devices.len();
    let shifts: Vec<VehicleShift> = devices
        .iter()
        .enumerate()
        .map(|(i, _d)| VehicleShift {
            start: ShiftStart {
                earliest: format!("2022-05-29T0{}:00:10Z", i),
                latest: None,
                location: (services[tour_size * i][0], services[tour_size * i][1]).to_loc(),
            },
            end: None,
            dispatch: None,
            breaks: None,
            reloads: None,
        })
        .collect();

    Problem {
        plan: Plan {
            clustering: None,
            jobs: services
                .clone()
                .into_iter()
                .enumerate()
                .map(|(i, lat_lon)| Job {
                    services: Some(vec![JobTask {
                        places: vec![JobPlace {
                            times: None,
                            location: (lat_lon[0], lat_lon[1]).to_loc(),
                            duration: 120.,
                            tag: None,
                        }],
                        demand: None,
                        order: None,
                    }]),
                    id: format!("{}", i).to_string(),
                    pickups: None,
                    deliveries: None,
                    replacements: None,
                    skills: None,
                    value: None,
                    group: None,
                    compatibility: None,
                })
                .collect::<Vec<_>>(),
            relations: None,
            areas: None,
        },
        objectives: None,
        fleet: Fleet {
            resources: None,
            profiles: vec![MatrixProfile {
                name: "normal_car".to_string(),
                speed: None,
            }],
            vehicles: vec![VehicleType {
                shifts,
                capacity: vec![1],
                type_id: "vehicle".to_string(),
                vehicle_ids: devices.clone(),
                costs: VehicleCosts {
                    fixed: Some(22.0),
                    distance: 0.0002,
                    time: 0.004806,
                },
                profile: VehicleProfile {
                    matrix: "normal_car".to_string(),
                    scale: None,
                },
                skills: None,
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    tour_size: Some(tour_size + 1),
                    areas: None,
                }),
            }],
        },
    }
}

fn get_core_problem(problem: Problem, matrices: Option<Vec<Matrix>>) -> Arc<CoreProblem> {
    Arc::new(
        if let Some(matrices) = matrices {
            (problem, matrices).read_pragmatic()
        } else {
            problem.read_pragmatic()
        }
        .unwrap(),
    )
}

fn get_core_solution<F: Fn(Arc<CoreProblem>) -> CoreSolution>(
    problem: Problem,
    matrices: Option<Vec<Matrix>>,
    solve_func: F,
) -> Solution {
    let core_problem = get_core_problem(problem, matrices);
    let core_solution = solve_func(core_problem.clone());

    let core_solution = sort_all_data(create_solution(&core_problem, &core_solution, None));

    sort_all_data(core_solution)
}

fn sort_all_data(solution: Solution) -> Solution {
    let mut solution = solution;

    solution
        .tours
        .sort_by(|a, b| a.vehicle_id.partial_cmp(&b.vehicle_id).unwrap_or(Less));

    if let Some(ref mut unassigned) = solution.unassigned {
        unassigned.sort_by(|a, b| a.job_id.partial_cmp(&b.job_id).unwrap_or(Less));
    }

    solution
}

pub fn solve(services: Vec<[f64; 2]>, generations: usize, devices: usize) -> Solution {
    let device_strings: Vec<String> = (0..devices).map(|i| format!("device_{}", i)).collect();
    let problem = create_problem(services, device_strings);
    get_core_solution(problem, None, |problem: Arc<CoreProblem>| {
        let environment = Arc::new(Environment::default());
        let telemetry_mode = get_default_telemetry_mode(environment.logger.clone());
        let (solution, _, _) =
            create_default_config_builder(problem.clone(), environment, telemetry_mode)
                .with_max_generations(Some(generations))
                .with_max_time(Some(60))
                .build()
                .map(|config| Solver::new(problem, config))
                .unwrap_or_else(|err| panic!("cannot build solver: {}", err))
                .solve()
                .unwrap_or_else(|err| panic!("cannot solve the problem: {}", err));
        solution
    })
}
