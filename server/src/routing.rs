use std::cmp::Ordering::Less;
use std::sync::Arc;
use vrp_pragmatic::core::models::Problem as CoreProblem;
use vrp_pragmatic::core::models::Solution as CoreSolution;
use vrp_pragmatic::core::solver::{create_default_config_builder, get_default_telemetry_mode, Solver};
use vrp_pragmatic::core::utils::Environment;
use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::problem::Fleet;
use vrp_pragmatic::format::problem::*;
use vrp_pragmatic::format::solution::{create_solution, Solution};
use vrp_pragmatic::format::{CoordIndex, Location};

trait ToLocation {
    fn to_loc(self) -> Location;
}

impl ToLocation for (f64, f64) {
    fn to_loc(self) -> Location {
        let (lat, lng) = self;
        Location::new_coordinate(lat, lng)
    }
}

fn create_job_place(location: (f64, f64), tag: Option<String>) -> JobPlace {
    JobPlace {
        times: None,
        location: location.to_loc(),
        duration: 1.,
        tag,
    }
}

fn create_job(id: &str) -> Job {
    Job {
        id: id.to_string(),
        pickups: None,
        deliveries: None,
        replacements: None,
        services: None,
        skills: None,
        value: None,
        group: None,
        compatibility: None,
    }
}

fn create_multi_job(id: &str, services: Vec<[f64; 2]>) -> Job {
    let create_tasks = |tasks: Vec<[f64; 2]>, prefix: &str| {
        let tasks = tasks
            .into_iter()
            .enumerate()
            .map(|(i, location)| JobTask {
                places: vec![JobPlace {
                    duration: 0.0,
                    ..create_job_place(
                        (location[0], location[1]),
                        Some(format!("{}{}", prefix, i + 1)),
                    )
                }],
                demand: None,
                order: None,
            })
            .collect::<Vec<_>>();

        if tasks.is_empty() {
            None
        } else {
            Some(tasks)
        }
    };

    Job {
        services: create_tasks(services, "s"),
        ..create_job(id)
    }
}

fn create_empty_plan() -> Plan {
    Plan {
        jobs: vec![],
        relations: None,
        areas: None,
        clustering: None,
    }
}

fn create_problem(services: Vec<[f64; 2]>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![create_multi_job("multi", services.clone())],
            ..create_empty_plan()
        },
        objectives: None,
        fleet: Fleet {
            profiles: vec![MatrixProfile {
                name: "car".to_string(),
                speed: None,
            }],
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "2022-05-29T00:00:10Z".to_string(),
                        latest: None,
                        location: (services[0][0], services[0][1]).to_loc(),
                    },
                    end: None,
                    dispatch: None,
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: (0., 0.).to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![2],
                type_id: "car".to_string(),
                vehicle_ids: vec!["car".to_string()],
                costs: VehicleCosts {
                    fixed: Some(22.0),
                    distance: 0.0002,
                    time: 0.004806,
                },
                profile: VehicleProfile {
                    matrix: "car".to_string(),
                    scale: None,
                },
                skills: None,
                limits: None,
            }],
        },
    }
}

fn create_matrix(data: Vec<i64>) -> Matrix {
    let size = (data.len() as f64).sqrt() as i32;

    assert_eq!((size * size) as usize, data.len());

    Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: data.clone(),
        distances: data.clone(),
        error_codes: None,
    }
}

fn create_matrix_from_problem(problem: &Problem) -> Matrix {
    let unique = CoordIndex::new(problem).unique();

    let data: Vec<i64> = unique
        .iter()
        .cloned()
        .flat_map(|a| {
            let (a_lat, a_lng) = a.to_lat_lng();
            unique.iter().map(move |b| {
                let (b_lat, b_lng) = b.to_lat_lng();
                ((a_lat - b_lat).powf(2.) + (a_lng - b_lng).powf(2.))
                    .sqrt()
                    .round() as i64
            })
        })
        .collect();

    create_matrix(data)
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
    perform_check: bool,
    solve_func: F,
) -> Solution {
    let format_problem = problem.clone();
    let format_matrices = matrices.clone();

    let core_problem = get_core_problem(problem, matrices);

    let core_solution = solve_func(core_problem.clone());

    let format_solution = sort_all_data(create_solution(&core_problem, &core_solution, None));

    if perform_check {
        if let Some(err) = CheckerContext::new(
            core_problem,
            format_problem.clone(),
            format_matrices,
            format_solution.clone(),
        )
        .and_then(|ctx| ctx.check())
        .err()
        {
            panic!(
                "check failed: '{}', problem: {:?}, solution: {:?}",
                err.join("\n"),
                format_problem,
                format_solution
            );
        }
    }

    sort_all_data(format_solution)
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

pub fn solve(services: Vec<[f64; 2]>, generations: usize) -> Solution {
    let problem = create_problem(services);
    let matrix = create_matrix_from_problem(&problem);
    get_core_solution(
        problem,
        Some(vec![matrix]),
        false,
        |problem: Arc<CoreProblem>| {
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
        },
    )
}
