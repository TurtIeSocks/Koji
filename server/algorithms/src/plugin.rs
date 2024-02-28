use std::fmt::Display;
use std::io::{self, BufRead, BufReader, Write};
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
    Clustering,
    Bootstrap,
}

impl Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Folder::Routing => write!(f, "routing"),
            Folder::Clustering => write!(f, "clustering"),
            Folder::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

#[derive(Debug)]
pub struct Plugin {
    plugin_path: String,
    interpreter: String,
    args: Vec<String>,
    pub plugin: String,
    pub split_level: u64,
}

pub type JoinFunction = fn(&Plugin, Vec<SingleVec>) -> SingleVec;

trait ParseCoord {
    fn parse_next_coord(&mut self) -> Option<f64>;
}

impl ParseCoord for std::str::Split<'_, &str> {
    fn parse_next_coord(&mut self) -> Option<f64> {
        if let Some(coord) = self.next() {
            if coord.contains(" ") {
                return coord.split(",").parse_next_coord();
            }
            if let Ok(coord) = coord.parse::<f64>() {
                return Some(coord);
            }
        }
        None
    }
}

impl Plugin {
    pub fn new(
        plugin: &str,
        folder: Folder,
        route_split_level: u64,
        input_args: &str,
    ) -> std::io::Result<Self> {
        let mut plugin_path = format!("algorithms/src/{folder}/plugins/{plugin}");
        if !Path::new(&plugin_path).exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("plugin {plugin} does not exist"),
            ));
        }
        let mut interpreter = match plugin.split(".").last() {
            Some("py") => "python3",
            Some("js") => "node",
            Some("sh") => "bash",
            Some("ts") => "ts-node",
            val => {
                if plugin == val.unwrap_or("") {
                    &plugin_path
                } else {
                    ""
                }
            }
        }
        .to_string();
        let args = input_args
            .split_whitespace()
            .skip_while(|arg| !arg.starts_with("--"))
            .map(|arg| arg.to_string())
            .collect::<Vec<String>>();

        for (index, pre_arg) in input_args
            .split_whitespace()
            .take_while(|arg| !arg.starts_with("--"))
            .enumerate()
        {
            log::info!("[PLUGIN PARSER] {index} | pre_arg: {}", pre_arg);
            if index == 0 {
                interpreter = pre_arg.to_string();
            } else if index == 1 {
                plugin_path = format!("algorithms/src/{folder}/plugins/{pre_arg}");
            } else {
                log::warn!("Unrecognized argument: {pre_arg} for plugin: {plugin}")
            }
        }

        if interpreter.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Unrecognized plugin, please create a PR to add support for it",
            ));
        };
        let path = Path::new(&plugin_path);
        if path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{plugin} is a directory, not a file, something may not be right with the provided args"),
            ));
        }
        if path.exists() {
            plugin_path = path.display().to_string();
            if interpreter == plugin_path {
                log::info!("{interpreter} {}", args.join(" "));
            } else {
                log::info!("{interpreter} {plugin_path} {}", args.join(" "));
            }
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
        }

        Ok(Plugin {
            plugin: plugin.to_string(),
            plugin_path,
            interpreter,
            split_level: route_split_level,
            args,
        })
    }

    pub fn run_multi<T>(
        &self,
        points: &SingleVec,
        joiner: Option<T>,
    ) -> Result<SingleVec, std::io::Error>
    where
        T: Fn(&Self, Vec<SingleVec>) -> SingleVec,
    {
        let handlers = if self.split_level == 0 {
            vec![self.run(utils::stringify_points(&points))?]
        } else {
            create_cell_map(&points, self.split_level)
                .into_values()
                .collect::<Vec<SingleVec>>()
                .into_par_iter()
                .filter_map(|x| self.run(utils::stringify_points(&x)).ok())
                .collect()
        };
        if let Some(joiner) = joiner {
            Ok(joiner(self, handlers))
        } else {
            Ok(handlers.into_iter().flatten().collect())
        }
    }

    pub fn run(&self, input: String) -> Result<SingleVec, std::io::Error> {
        log::info!("spawning {} child process", self.plugin);

        let time = Instant::now();

        let mut child = Command::new(&self.interpreter);
        if self.plugin_path != self.interpreter {
            child.arg(&self.plugin_path);
        };
        let mut child = match child
            .args(self.args.iter())
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
                    "Failed to open stdin",
                ));
            }
        };

        std::thread::spawn(move || match stdin.write_all(input.as_bytes()) {
            Ok(_) => match stdin.flush() {
                Ok(_) => {}
                Err(err) => {
                    log::error!("failed to flush stdin: {}", err);
                }
            },
            Err(err) => {
                log::error!("failed to write to stdin: {}", err)
            }
        });

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not capture stdout"))?;

        let mut results = vec![];
        let mut invalid = vec![];
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.contains(" ") {
                        let mut iter: std::str::Split<'_, &str> = line.trim().split(" ");
                        while let Some(line) = iter.next() {
                            let mut coord: std::str::Split<'_, &str> = line.trim().split(",");
                            let lat = coord.parse_next_coord();
                            let lng = coord.parse_next_coord();
                            if lat.is_none() || lng.is_none() {
                                invalid.push(line.to_string())
                            } else {
                                results.push([lat.unwrap(), lng.unwrap()])
                            }
                        }
                    } else {
                        let mut iter: std::str::Split<'_, &str> = line.trim().split(",");
                        let lat = iter.parse_next_coord();
                        let lng = iter.parse_next_coord();
                        if lat.is_none() || lng.is_none() {
                            invalid.push(line)
                        } else {
                            results.push([lat.unwrap(), lng.unwrap()])
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading line: {}", e);
                }
            }
        }

        match child.wait()? {
            status if status.success() => {}
            status => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("child process exited with status: {}", status),
                ))
            }
        }

        if let Some(first) = results.first() {
            if let Some(last) = results.last() {
                if first == last {
                    results.pop();
                }
            }
        }

        if !invalid.is_empty() {
            log::warn!(
                "Some invalid results were returned from the plugin: `{}`",
                invalid.join(", ")
            );
        }
        if results.is_empty() {
            Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "no valid output from child process \n{}\noutput should return points in the following format: `lat,lng lat,lng`",
                        invalid.join(", ")
                    ),
                ))
        } else {
            log::info!(
                "{} child process finished in {}s with {} points",
                self.plugin,
                time.elapsed().as_secs_f32(),
                results.len()
            );
            // Ok(output_indexes.into_iter().map(|i| points[i]).collect())
            Ok(results)
        }
    }
}
