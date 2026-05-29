use macros::time;
use model::api::single_vec::SingleVec;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;

use crate::{clustering::candidates, rtree};

#[derive(Clone, Debug)]
struct Individual {
    clusters: Vec<[f64; 2]>,
    fitness: f64,
}

pub struct GeneticClusterOptimizer {
    data_points: Vec<[f64; 2]>, // [latitude, longitude]
    min_points: usize,
    max_clusters: usize,
    radius: f64, // radius in meters
    population_size: usize,
    generations: usize,
    mutation_rate: f64,
    crossover_rate: f64,
}

impl GeneticClusterOptimizer {
    pub fn new(
        data_points: Vec<[f64; 2]>,
        min_points: usize,
        max_clusters: usize,
        radius: f64,
    ) -> Self {
        Self {
            data_points,
            min_points,
            max_clusters,
            radius,
            population_size: 100,
            generations: 200,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
        }
    }

    #[time("genetic optimizing")]
    pub fn optimize(&self, initial_clusters: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
        let mut rng = rand::rng();

        // Initialize population (coverage-aware, bounded, deduped)
        let mut population = self.initialize_population(&initial_clusters, &mut rng);

        for generation in 0..self.generations {
            // Evaluate fitness
            population.par_iter_mut().for_each(|individual| {
                individual.fitness = self.calculate_fitness(&individual.clusters);
            });

            // Sort by fitness (lower is better)
            population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());

            if generation % 20 == 0 {
                log::info!(
                    "Generation {}: Best fitness = {:.2}, Clusters = {}",
                    generation,
                    population[0].fitness,
                    population[0].clusters.len()
                );
            }

            // Elitism
            let elite_count = self.population_size / 10;
            let mut new_population = Vec::with_capacity(self.population_size);
            new_population.extend_from_slice(&population[0..elite_count]);

            // Offspring
            let offspring_count = self.population_size - elite_count;
            let offspring: Vec<Individual> = (0..offspring_count)
                .into_par_iter()
                .map(|_| {
                    let mut rng = rand::rng();
                    let parent1 = self.tournament_selection(&population, &mut rng);
                    let parent2 = self.tournament_selection(&population, &mut rng);

                    let mut child = if rng.random::<f64>() < self.crossover_rate {
                        self.crossover(parent1, parent2, &mut rng)
                    } else {
                        parent1.clone()
                    };

                    self.mutate(&mut child, &mut rng);
                    self.repair(&mut child.clusters);

                    child
                })
                .collect();

            new_population.extend(offspring);
            population = new_population;
        }

        // Final evaluation
        population.par_iter_mut().for_each(|individual| {
            individual.fitness = self.calculate_fitness(&individual.clusters);
        });
        population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());

        let best = &population[0];
        let covered = self.count_covered_points(&best.clusters);
        log::info!("Optimization complete!");
        log::info!("Final best fitness: {:.2}", best.fitness);
        log::info!("Total clusters: {}", best.clusters.len());
        log::info!("Points covered: {} / {}", covered, self.data_points.len());
        log::info!("Uncovered points: {}", self.data_points.len() - covered);

        best.clusters.clone()
    }

    /* -------------------------- core GA helpers -------------------------- */

    fn initialize_population(
        &self,
        initial_clusters: &[[f64; 2]],
        rng: &mut impl Rng,
    ) -> Vec<Individual> {
        // Make a "repaired" seed first
        let mut seed = initial_clusters.to_vec();
        self.repair(&mut seed);

        let mut population = Vec::with_capacity(self.population_size);

        // Add the repaired initial solution
        population.push(Individual {
            clusters: seed.clone(),
            fitness: 0.0,
        });

        // Build diverse, coverage-aware variations
        for _ in 1..self.population_size {
            // Start from a lightly perturbed copy of the seed
            let mut clusters = seed.clone();
            for c in &mut clusters {
                if rng.random::<f64>() < 0.6 {
                    let (dlat, dlng) = self.random_offset_meters(self.radius * 0.6, c[0], rng);
                    c[0] += dlat;
                    c[1] += dlng;
                    self.clamp_coordinates(c);
                }
            }

            // Greedily add uncovered points until near capacity
            self.repair(&mut clusters);
            let mut uncovered = self.get_uncovered_points(&clusters);
            // Randomize order to avoid deterministic greediness
            if !uncovered.is_empty() {
                // Reservoir-sample some uncovered points to avoid bias
                let target_add = self.max_clusters.saturating_sub(clusters.len()).min(8); // keep diversity; don't fill completely
                for _ in 0..target_add {
                    let idx = rng.random_range(0..uncovered.len());
                    clusters.push(self.data_points[uncovered[idx]]);
                    uncovered.swap_remove(idx);
                    if clusters.len() >= self.max_clusters {
                        break;
                    }
                }
            }

            // Add a *small* random subset of candidates instead of all 256
            let candidates: Vec<[f64; 2]> =
                candidates::generate_clusters_from_points(&self.data_points, self.radius, 32);
            for c in candidates {
                if rng.random::<f64>() < 0.25 && clusters.len() < self.max_clusters {
                    let mut cc = c;
                    if rng.random::<f64>() < 0.4 {
                        // slight jitter to avoid duplicates
                        let (dlat, dlng) = self.random_offset_meters(self.radius * 0.3, cc[0], rng);
                        cc[0] += dlat;
                        cc[1] += dlng;
                        self.clamp_coordinates(&mut cc);
                    }
                    clusters.push(cc);
                }
            }

            self.repair(&mut clusters);
            population.push(Individual {
                clusters,
                fitness: 0.0,
            });
        }

        population
    }

    // Order-invariant, deduped crossover
    fn crossover(
        &self,
        parent1: &Individual,
        parent2: &Individual,
        rng: &mut impl Rng,
    ) -> Individual {
        let mut combined = Vec::new();

        // Random subset from P1
        for c in &parent1.clusters {
            if rng.random::<f64>() < 0.6 {
                combined.push(*c);
            }
        }
        // Random subset from P2
        for c in &parent2.clusters {
            if rng.random::<f64>() < 0.6 {
                combined.push(*c);
            }
        }

        // If we took too few (possible), pad with some from parents
        if combined.is_empty() {
            if !parent1.clusters.is_empty() {
                combined.push(parent1.clusters[rng.random_range(0..parent1.clusters.len())]);
            }
            if !parent2.clusters.is_empty() && combined.len() < 2 {
                combined.push(parent2.clusters[rng.random_range(0..parent2.clusters.len())]);
            }
        }

        // Repair will dedup and cap
        let mut child = Individual {
            clusters: combined,
            fitness: 0.0,
        };
        self.repair(&mut child.clusters);
        child
    }

    fn mutate(&self, individual: &mut Individual, rng: &mut impl Rng) {
        // Move some clusters
        for cluster in &mut individual.clusters {
            if rng.random::<f64>() < self.mutation_rate {
                let (lat_offset, lng_offset) =
                    self.random_offset_meters(self.radius * 0.5, cluster[0], rng);
                cluster[0] += lat_offset;
                cluster[1] += lng_offset;
                self.clamp_coordinates(cluster);
            }
        }

        // Remove a cluster that currently covers nothing (if any), else random
        if !individual.clusters.is_empty() && rng.random::<f64>() < self.mutation_rate * 0.6 {
            let useless = self.find_useless_clusters(&individual.clusters);
            if let Some(idx) = useless
                .and_then(|v| if v.is_empty() { None } else { Some(v) })
                .and_then(|v| Some(v[rng.random_range(0..v.len())]))
            {
                let _ = individual.clusters.remove(idx);
            } else {
                let idx = rng.random_range(0..individual.clusters.len());
                let _ = individual.clusters.remove(idx);
            }
        }

        // Add a new cluster at an uncovered point
        if individual.clusters.len() < self.max_clusters && rng.random::<f64>() < self.mutation_rate
        {
            let uncovered = self.get_uncovered_points(&individual.clusters);
            if !uncovered.is_empty() {
                let point_idx = rng.random_range(0..uncovered.len());
                let mut new_cluster = self.data_points[uncovered[point_idx]];
                // small jitter so it's inside radius neighborhoods more flexibly
                if rng.random::<f64>() < 0.4 {
                    let (dlat, dlng) =
                        self.random_offset_meters(self.radius * 0.25, new_cluster[0], rng);
                    new_cluster[0] += dlat;
                    new_cluster[1] += dlng;
                    self.clamp_coordinates(&mut new_cluster);
                }
                individual.clusters.push(new_cluster);
            }
        }

        // Replace a cluster with an uncovered point
        if !individual.clusters.is_empty() && rng.random::<f64>() < self.mutation_rate * 0.4 {
            let uncovered = self.get_uncovered_points(&individual.clusters);
            if !uncovered.is_empty() {
                let cluster_idx = rng.random_range(0..individual.clusters.len());
                individual.clusters[cluster_idx] =
                    self.data_points[uncovered[rng.random_range(0..uncovered.len())]];
            }
        }

        // Final repair to keep bounds & uniqueness
        self.repair(&mut individual.clusters);
    }

    /* -------------------------- fitness & utils -------------------------- */

    fn calculate_fitness(&self, clusters: &SingleVec) -> f64 {
        let covered_points = self.count_covered_points(clusters);
        let total_points = self.data_points.len();
        let total_clusters = clusters.len();
        let uncovered_points = total_points - covered_points;

        // Lower is better: penalize uncovered, then cluster count.
        // (If this still prefers too-few clusters, reduce the weight below.)
        (total_clusters * self.min_points + uncovered_points) as f64
    }

    fn count_covered_points(&self, clusters: &SingleVec) -> usize {
        let mut covered = HashSet::new();

        let cluster_tree = rtree::spawn(self.radius, clusters);
        for (point_idx, point) in self.data_points.iter().enumerate() {
            if cluster_tree.locate_at_point(point).is_some() {
                covered.insert(point_idx);
            }
        }

        covered.len()
    }

    fn get_uncovered_points(&self, clusters: &SingleVec) -> Vec<usize> {
        let mut uncovered = Vec::new();

        let cluster_tree = rtree::spawn(self.radius, clusters);
        for (point_idx, point) in self.data_points.iter().enumerate() {
            if cluster_tree.locate_at_point(point).is_none() {
                uncovered.push(point_idx);
            }
        }

        uncovered
    }

    fn find_useless_clusters(&self, clusters: &SingleVec) -> Option<Vec<usize>> {
        if clusters.is_empty() {
            return Some(vec![]);
        }

        let tree = rtree::spawn(self.radius, &self.data_points);
        // For each cluster, count how many points it *uniquely* covers
        // (approximate: if a point is covered by any cluster, attribute to the first hit)
        // let tree = rtree::spawn(self.radius, clusters);
        // let mut counts: HashMap<usize, usize> = HashMap::new();

        // for point in &self.data_points {
        //     if let Some(idx) = tree.locate_at_point(point) {
        //         *counts.entry(idx.).or_insert(0) += 1;
        //     }
        // }

        let mut useless = Vec::new();
        for (i, c) in clusters.iter().enumerate() {
            if tree.locate_at_point(c).is_none() {
                useless.push(i);
            }
        }
        Some(useless)
    }

    /// Convert meters to approximate latitude/longitude offsets
    fn meters_to_lat_lng(&self, meters: f64, latitude: f64) -> (f64, f64) {
        const METERS_PER_DEGREE_LAT: f64 = 111320.0;
        let meters_per_degree_lng = 111320.0 * latitude.to_radians().cos();

        let lat_offset = meters / METERS_PER_DEGREE_LAT;
        let lng_offset = meters / meters_per_degree_lng.max(1e-9); // guard against poles

        (lat_offset, lng_offset)
    }

    /// Generate a random offset in meters, converted to lat/lng
    fn random_offset_meters(
        &self,
        max_meters: f64,
        latitude: f64,
        rng: &mut impl Rng,
    ) -> (f64, f64) {
        let angle = rng.random::<f64>() * 2.0 * std::f64::consts::PI;
        let distance = rng.random::<f64>() * max_meters;

        let (lat_per_meter, lng_per_meter) = self.meters_to_lat_lng(1.0, latitude);

        let lat_offset = (distance * angle.sin()) * lat_per_meter;
        let lng_offset = (distance * angle.cos()) * lng_per_meter;

        (lat_offset, lng_offset)
    }

    /// Clamp coordinates to valid lat/lng ranges
    fn clamp_coordinates(&self, coords: &mut [f64; 2]) {
        coords[0] = coords[0].clamp(-90.0, 90.0);
        coords[1] = if coords[1] > 180.0 {
            coords[1] - 360.0
        } else if coords[1] < -180.0 {
            coords[1] + 360.0
        } else {
            coords[1]
        };
    }

    fn tournament_selection<'a>(
        &'a self,
        population: &'a [Individual],
        rng: &mut impl Rng,
    ) -> &'a Individual {
        let tournament_size = 5;
        let mut best = &population[rng.random_range(0..population.len())];

        for _ in 1..tournament_size {
            let candidate = &population[rng.random_range(0..population.len())];
            if candidate.fitness < best.fitness {
                best = candidate;
            }
        }

        best
    }

    /* ---------------------------- repair step ---------------------------- */

    fn repair(&self, clusters: &mut Vec<[f64; 2]>) {
        // 1) Deduplicate with a small epsilon in degrees
        let eps = 1e-5_f64; // ~1 meter in lat; ok for dedup
        let mut seen = HashSet::<(i64, i64)>::new();
        clusters.retain(|c| {
            let key = ((c[0] / eps).round() as i64, (c[1] / eps).round() as i64);
            seen.insert(key)
        });

        // 2) Enforce max_clusters (random prune; could be improved with marginal coverage)
        while clusters.len() > self.max_clusters {
            // Prefer dropping clusters that cover nothing
            if let Some(useless) = self.find_useless_clusters(clusters) {
                if !useless.is_empty() {
                    let idx = useless[0];
                    clusters.remove(idx);
                    continue;
                }
            }
            clusters.pop();
        }
    }
}

/* ------------------ rtree additions used above (hinted) ------------------
Add this tiny convenience on your rtree wrapper if it doesn't exist yet:

impl RTreeWrapper {
    pub fn locate_at_point_with_index(&self, point: &[f64;2]) -> Option<usize> {
        // return the index of *a* cluster covering the point
        // (mirror your locate_at_point but include index)
    }
}
*/
