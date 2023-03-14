# Kōji

[![GitHub Release](https://img.shields.io/github/release/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/releases/)

<!-- [![GitHub Contributors](https://img.shields.io/github/contributors/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/graphs/contributors/) -->

[![Discord](https://img.shields.io/discord/907337201044582452.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/EYYsKPVawn)

## Features

- Geofence manager, editor, and distribution tool
- API based clustering for automatic route making
- Integrated with OR-Tools for fast route calculating
- Geofence conversions into various formats
- Interactive client for creating and managing geofences

## Data Compatibility

- [RealDeviceMap (RDM)](https://github.com/realdevicemap/realdevicemap)
- [Golbat](https://github.com/UnownHash/Golbat)

# Installation

## Docker (Recommended)

1. `docker login ghcr.io/turtiesocks/koji`
2. Enter your GitHub username
3. Enter your GitHub authentication token
4. If you don't already have a `docker-compose.yml` file, create one with `touch docker-compose.yml`
5. Copy the contents of the example [docker-compose.yml](./docker-compose.example.yml) file
6. `nano docker-compose.yml`
7. Paste the copied contents
8. Set the env variables as above
9. `docker-compose pull`
10. `docker-compose up -d`

## Standard

1. Clone this repo:

```bash
git clone https://github.com/TurtIeSocks/Koji.git
```

2. [Install NodeJS](https://nodejs.dev/en/learn/how-to-install-nodejs/)
3. [Install Rust](https://www.rust-lang.org/tools/install)
4. Install `curl` for your system if it's not already present
5. Install OR-Tools

   - [Check PreReqs](https://developers.google.com/optimization/install/cpp/binary_linux#prerequisites)

   For example, on Ubuntu 20.04:

   ```bash
   sudo apt update
   sudo apt install -y build-essential cmake lsb-release
   ```

   - Run the install script:

   ```bash
   sudo chmod +x or-tools/install.sh && ./or-tools/install.sh
   ```

6. Copy the env file:

```bash
cd server && cp .env.example .env
```

8. Edit the env file: `nano .env`
   - Set the `SCANNER_DB_URL` to your RDM database url
   - Set the `KOJI_DB_URL` to the database you want Kōji to write migrations to
   - Set `KOJI_SECRET` to your preferred secret, this will be used for the bearer token when calling the API and logging into the client
   - Set `START_LAT` and `START_LON` to wherever you want the map to start
9. Compile the client:

```bash
cd ../client && yarn install && yarn build
```

10. Compile the server:

```bash
cd ../server && cargo run -r
# you might have to also install pkg-config (`apt install pkg-config`)
```

11. Optionally install [pm2](https://pm2.keymetrics.io/) to run the server in the background:

```bash
  npm install pm2 -g
  pm2 start "cargo run -r" --name koji # from the /server folder
```

## Using the Client

1. Open a browser with whatever port you specified in the `.env` or `docker-compose.yml` file. (default is 8080)
2. Login with the secret you set as your `KOJI_SECRET`, or press enter to login with no password if you didn't set it.

## Updating

Docker:

1. `docker-compose pull`
2. `docker-compose down && docker-compose up -d`

Local Repo:

1. Pull update
   - `git pull`
2. Recompile OR-Tools:
   - `./ort-tools/install.sh`
3. Recompile Client:
   - `cd client && yarn install && yarn build`
4. Recompile Server
   - `cd ../server && cargo run -r`
5. If using pm2:
   - `pm2 restart koji`

## Development

1. After going through the standard setup instructions
2. Install Cargo Watch:

```bash
cargo install cargo-watch
```

3. Install the VS Code Plugin [Rust Analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer)
4. Open one terminal:

```bash
cd server
# to compile in debug mode (faster recompiling but slower performance)
DEBUG=true cargo watch -x run
# to compile in release mode (slower recompiling but faster performance)
cargo watch -x 'run -r'
```

5. In another terminal:

```bash
cd client && yarn install && yarn dev
```

6. A browser will automatically open to `localhost:{PORT}`

# API

## General Types:

### Data Structs, Type Aliases, and Enums

```rust
pub type PointArray<T = f64> = [T; 2]; // [lat, lon]
pub type SingleVec<T = f64> = Vec<PointArray<T>>;
pub type MultiVec<T = f64> = Vec<Vec<PointArray<T>>>;

pub struct PointStruct<T: Float = f64> {
    pub lat: T,
    pub lon: T,
}
pub type SingleStruct<T = f64> = Vec<PointStruct<T>>;
pub type MultiStruct<T = f64> = Vec<Vec<PointStruct<T>>>;

pub struct BoundsArg {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

// Accepted Area Inputs and Outputs:
pub enum GeoFormats {
    Bounds(BoundsArg),
    Text(String),
    // can be either:
      // lat,lon\nlat,lon
      // or lat lon,lat lon
    SingleArray(SingleVec),
    MultiArray(MultiVec),
    SingleStruct(SingleStruct),
    MultiStruct(MultiStruct),
    Feature(Feature),
    FeatureVec(Vec<Feature>),
    FeatureCollection(FeatureCollection),
    Poracle(Poracle),
}

// Return Types:
pub enum ReturnType {
  AltText,            // lat lon,lat lon
  Text,               // lat,lon\nlat,lon
  SingleArray,        // [[lat, lon]]
  MultiArray,         // [[[lat, lon]]]
  SingleStruct,       // [{lat: lat, lon: lon}]
  MultiStruct,        // [[{lat: lat, lon: lon}]]
  Feature,            // GeoJSON Feature ([lon, lat])
  FeatureVec,         // [Feature]
  FeatureCollection,  // GeoJSON FeatureCollection
  Poracle             // Poracle Geo Format ([lat, lon])
}

// Sort by types, only valid when clustering in non-fast mode:
pub enum SortBy {
    GeoHash,      // default, sorts by geohash, fastest alternative to okay routing
    ClusterCount, // sorts by the number of data points in each cluster
    Random,       // randomizes the order of the clusters
}

// Data Input Types:
pub enum DataPointsArg {
    Array(SingleVec),
    Struct(SingleStruct),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
}

// Spawnpoint Args:
pub enum SpawnpointTth {
  All,      // All spawnpoints
  Known,    // Only spawnpoints with known TTH
  Unknown,  // Only spawnpoints with unknown TTH
}
```

### API Args

```rust
pub struct Args {
    /// The area input to be used for data point collection.
    ///
    /// Accepts an optional [GeoFormats]
    ///
    /// Default: `None`
    pub area: Option<GeoFormats>,
    /// Only returns stats from the API
    ///
    /// Default: `false`
    pub benchmark_mode: Option<bool>,
    /// Data points to cluster or reroute.
    /// Overrides any inputted area.
    ///
    /// Accepts [DataPointsArg]
    pub data_points: Option<DataPointsArg>,
    /// Number of devices to use in VRP routing
    ///
    /// Default: `1`
    ///
    /// Deprecated
    pub devices: Option<usize>,
    /// Whether to use the fast or slow clustering algorithm
    ///
    /// Default: `true`
    pub fast: Option<bool>,
    /// Number of times to run through a clustering algorithm
    ///
    /// Default: `0`
    ///
    /// Deprecated
    pub generations: Option<usize>,
    /// Geometry type used during conversions
    ///
    /// Currently unstable and will likely change how it's used
    pub geometry_type: Option<String>,
    /// Name used for geofence lookup.
    /// Tries the Kōji database first.
    /// Then checks the scanner database if it doesn't find one.
    pub instance: Option<String>,
    /// Last seen date timestamp for filtering data points from the database.
    ///
    /// Default: `0`
    pub last_seen: Option<u32>,
    /// Internally used, unstable
    pub mode: Option<String>,
    /// Minimum number of points to use in the clustering algorithms
    ///
    /// Default: `1`
    pub min_points: Option<usize>,
    /// Only counts min_points by the number of unique data_points that a cluster covers.
    /// Only available when `fast: false`
    pub only_unique: Option<bool>,
    /// Radius of the circle to be used in clustering/routing,
    /// in meters
    ///
    /// Default: `70`
    pub radius: Option<Precision>,
    /// The return type for the data
    ///
    /// Accepts [ReturnTypeArg]
    ///
    /// Default: `SingleVec`
    pub return_type: Option<String>,
    /// Manual chunking to split TSP routing.
    ///
    /// Default: 1
    ///
    /// Deprecated
    pub route_chunk_size: Option<usize>,
    /// Geohash precision level for splitting up routing into multiple threads
    ///
    /// Recommend using 4 for Gyms, 5 for Pokestops, and 6 for Spawnpoints
    ///
    /// Default: `1`
    pub route_split_level: Option<usize>,
    /// Amount of time for the TSP solver to run
    ///
    /// Default: `0` (auto)
    ///
    /// Deprecated
    pub routing_time: Option<i64>,
    /// Saves the calculated route to the Kōji database
    ///
    /// Default: `false`
    pub save_to_db: Option<bool>,
    /// Saves the calculated route to the scanner database
    ///
    /// Default: `false`
    pub save_to_scanner: Option<bool>,
    /// Simplifies Polygons and MultiPolygons when converting them
    ///
    /// Default: `false`
    pub simplify: Option<bool>,
    /// Sorts *clustering* results, not routing results.
    /// This is just intended to do some simple clustering adjustments,
    /// when you don't need a full TSP solver
    ///
    /// Accepts [SortBy] - case sensitive
    ///
    /// Default: `GeoHash`
    pub sort_by: Option<SortBy>,
    /// Filter spawnpoints by confirmed, unconfirmed, or all
    ///
    /// Accepts [SpawnpointTth] - case sensitive
    ///
    /// Default: `All`
    pub tth: Option<SpawnpointTth>,
}
```

### Return Structs

```rust
// Standard Response Struct (what you will receive!)
  pub struct Response {
      pub message: String,
      pub status: String,
      pub status_code: u16,
      pub data: GeoFormats,
      pub stats: Stats,
  }

// Benchmark/Stats
  pub struct Stats {
      pub best_clusters: SingleVec,
      pub best_cluster_point_count: usize,
      pub cluster_time: f32,
      pub total_points: usize,
      pub points_covered: usize,
      pub total_clusters: usize,
      pub total_distance: f64,
      pub longest_distance: f64,
  }
```

## Data Endpoints

### /api/v1/geofence/{ReturnType}

- **Method:** `GET`
- **URL Params**:
  - See `ReturnType` enum above
- **Returns**:
  - All geofences saved in the Kōji database in the format specified by `ReturnType`

### /api/v1/geofence/{ReturnType}/{Project_Name}

- **Method:** `GET`
- **URL Params**:
  - See `ReturnType` enum above
  - Name of a saved project in the Kōji database
- **Returns**:
  - The geofences saved in the Kōji database in the format specified by `ReturnType` that are related to the specified `Project_Name`

### /api/v1/route/{ReturnType}

- **Method:** `GET`
- **URL Params**:
  - See `ReturnType` enum above
- **Returns**:
  - All routes saved in the Kōji database in the format specified by `ReturnType`

### /api/v1/route/{ReturnType}/{Project_Name}

- **Method:** `GET`
- **URL Params**:
  - See `ReturnType` enum above
  - Name of a saved project in the Kōji database
- **Returns**:
  - The routes saved in the Kōji database in the format specified by `ReturnType` that are related to the specified `Project_Name`

## Calculation Endpoints

### /api/v1/calc/bootstrap

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance`
  - **Optional**:
    - `radius`
    - `return_type`
    - `instance`
    - `save_to_db`
    - `save_to_scanner`
    - `benchmark_mode`
- **Returns**:
  - Bootstrap route data for the specified area/instance with the specified radius

### /api/v1/calc/cluster

### /api/v1/calc/route

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance` OR `data_points`
  - **Optional**:
    - `radius`
    - `return_type`
    - `min_points`
    - `generations`
    - `devices`
    - `fast`
    - `only_unique`
    - `instance`
    - `save_to_db`
    - `save_to_scanner`
    - `benchmark_mode`
    - `sort_by`
    - `tth`
    - `route_split_level`
    - `last_seen`
- **Returns**:
  - Clustered/routing data for the specified area/instance with the specified radius

### /api/v1/calc/reroute

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `data_points`
  - **Optional**:
    - `return_type`
    - `instance`
    - `save_to_db`
    - `save_to_scanner`
    - `benchmark_mode`
    - `route_split_level`
- **Returns**:
  - Rerouted data for the `data_points` specified

### /api/v1/calc/area

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area`
- **Returns**:
  - Returns the total area of the specified geofence(s) `{ "area": f64 }`

## Conversions & Helpers

### /api/v1/convert/data

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area`
  - **Optional**:
    - `return_type`
- **Returns**:
  - Converted data points in any of the supported formats

### /api/v1/convert/simplify

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area`
  - **Optional**:
    - `return_type`
- **Returns**:
  - Simplify Polygons and MultiPolygons

### /api/v1/convert/merge-points

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area`
  - **Optional**:
    - `return_type`
- **Returns**:
  - Merges points into a GeoJSON MultiPoint feature

## S2 Cells

### /api/v1/s2/{cell_level}

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `BoundsArg`
- **Returns**:
  - Returns S2 cells found in the provided bounds at the provided level
