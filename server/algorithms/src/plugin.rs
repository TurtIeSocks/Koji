use std::fmt::Display;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::s2::create_cell_map;
use crate::utils;
use model::api::single_vec::SingleVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[derive(Debug)]
pub enum Folder {
    Routing,
    // Sorting,
    // Bootstrap,
}

impl Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Folder::Routing => write!(f, "routing"),
            // Folder::Sorting => write!(f, "sorting"),
            // Folder::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

#[derive(Debug)]
pub struct Plugin {
    plugin_path: String,
    interpreter: String,
    radius: f64,
    pub plugin: String,
    pub split_level: u64,
}

pub type JoinFunction = fn(&Plugin, Vec<SingleVec>) -> SingleVec;

impl Plugin {
    pub fn new(
        plugin: &str,
        folder: Folder,
        route_split_level: u64,
        radius: f64,
    ) -> std::io::Result<Self> {
        let path = format!("algorithms/src/{folder}/plugins/{plugin}");
        let path = Path::new(path.as_str());
        let plugin_path = if path.exists() {
            path.display().to_string()
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "{plugin} does not exist{}",
                    if plugin == "tsp" {
                        ", rerun the OR Tools Script"
                    } else {
                        ""
                    }
                ),
            ));
        };

        let interpreter = match plugin.split(".").last() {
            Some("py") => "python3",
            Some("js") => "node",
            Some("ts") => "ts-node",
            val => {
                if plugin == val.unwrap_or("") {
                    ""
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Unrecognized plugin, please create a PR to add support for it",
                    ));
                }
            }
        };
        Ok(Plugin {
            plugin: plugin.to_string(),
            plugin_path,
            interpreter: interpreter.to_string(),
            split_level: route_split_level,
            radius,
        })
    }

    pub fn run<T>(&self, points: &SingleVec, joiner: Option<T>) -> Result<SingleVec, std::io::Error>
    where
        T: Fn(&Self, Vec<SingleVec>) -> SingleVec,
    {
        let handlers = if self.split_level == 0 {
            vec![self.spawn(&points)?]
        } else {
            create_cell_map(&points, self.split_level)
                .into_values()
                .collect::<Vec<SingleVec>>()
                .into_par_iter()
                .filter_map(|x| self.spawn(&x).ok())
                .collect()
        };
        if let Some(joiner) = joiner {
            Ok(joiner(self, handlers))
        } else {
            Ok(handlers.into_iter().flatten().collect())
        }
    }

    fn spawn(&self, points: &SingleVec) -> Result<SingleVec, std::io::Error> {
        log::info!("spawning {} child process", self.plugin);

        let time = Instant::now();
        let stringified_points = utils::stringify_points(&points);

        let mut child = if self.interpreter.is_empty() {
            Command::new(&self.plugin_path)
        } else {
            Command::new(&self.interpreter)
        };
        if !self.interpreter.is_empty() {
            child.arg(&self.plugin_path);
        }
        let mut child = match child
            .args(&["--input", &stringified_points])
            .args(&["--radius", &self.radius.to_string()])
            .args(&["--route_split_level", &self.split_level.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => return Err(err),
        };

        let mut stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to open stdin",
                ));
            }
        };

        match stdin.write_all(stringified_points.as_bytes()) {
            Ok(_) => match stdin.flush() {
                Ok(_) => {}
                Err(err) => {
                    log::error!("failed to flush stdin: {}", err);
                }
            },
            Err(err) => {
                log::error!("failed to write to stdin: {}", err)
            }
        };

        let output = match child.wait_with_output() {
            Ok(result) => result,
            Err(err) => return Err(err),
        };
        let output = String::from_utf8_lossy(&output.stdout);
        let mut output_indexes = output
            .split(",")
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .collect::<Vec<usize>>();
        if let Some(first) = output_indexes.first() {
            if let Some(last) = output_indexes.last() {
                if first == last {
                    output_indexes.pop();
                }
            }
        }
        if output_indexes.is_empty() {
            Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "no valid output from child process {}, output should return comma separated indexes of the input clusters in the order they should be routed",
                        output
                    ),
                ))
        } else {
            log::info!(
                "{} child process finished in {}s",
                self.plugin,
                time.elapsed().as_secs_f32()
            );
            Ok(output_indexes.into_iter().map(|i| points[i]).collect())
        }
    }
}
