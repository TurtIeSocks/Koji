# Kōji

<!-- [![GitHub Release](https://img.shields.io/github/release/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/releases/)
[![GitHub Contributors](https://img.shields.io/github/contributors/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/graphs/contributors/) -->

[![Discord](https://img.shields.io/discord/907337201044582452.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/EYYsKPVawn)

## Features

- Geofence manager, editor, and distribution tool
- API based clustering for automatic route making
- Fast route solver for quest scanning
- Geofence conversions into various formats

## Data Compatibility

- [RealDeviceMap (RDM)](https://github.com/realdevicemap/realdevicemap)
- [Golbat](https://github.com/UnownHash/Golbat)

## Installation

### Docker (Recommended)

### Temp Authenticate

1. `docker login ghcr.io/turtiesocks/koji`
2. Enter your GitHub username
3. Enter your GitHub authentication token

4. If you don't already have a `docker-compose.yml` file, `touch docker-compose.yml`
5. Copy the contents of the example [docker-compose.yml](./docker-compose.example.yml) file
6. `nano docker-compose.yml`
7. Paste the copied contents
8. Set the env variables as above
9. `docker-compose pull`
10. `docker-compose up -d`

### Standard

1. Clone this repo:

```bash
git clone https://github.com/TurtIeSocks/Koji.git
```

2. [Install NodeJS](https://nodejs.dev/en/learn/how-to-install-nodejs/)
3. [Install Rust](https://www.rust-lang.org/tools/install)
4. Install OR-Tools
5. [Check PreReqs](https://developers.google.com/optimization/install/cpp/binary_linux#prerequisites)

For example, on Ubuntu 20.04:

```bash
sudo apt update
sudo apt install -y build-essential cmake lsb-release
```

6. Run the install script:

```bash
sudo chmod +x or-tools/install.sh && ./or-tools/install.sh
```

7. Copy the env file:

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

### Using the Client

1. Open a browser with whatever port you specified in the `.env` or `docker-compose.yml` file. (default is 8080)
2. Login with the secret you set as your `KOJI_SECRET`, or press enter to login with no password if you didn't set it.

### Updating

Local Repo:

1. `git pull`
2. `cd client && yarn install && yarn build`
3. `cd ../server && cargo run -r`
4. If using pm2, `pm2 restart koji`

Docker:

1. `docker-compose pull`
2. `docker-compose down && docker-compose up -d`

### Development

1. After installing Rust and Node
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

## API

General Types:

```rust
// Data Structs and Type Aliases
pub type PointArray<T = f64> = [T; 2];
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
    AltText, // lat lon,lat lon
    Text, // lat,lon\nlat,lon
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Feature,
    FeatureVec,
    FeatureCollection,
    Poracle
}

// Data Input Types:
  pub enum DataPointsArg {
      Array(SingleVec),
      Struct(SingleStruct),
      Feature(Feature),
      FeatureCollection(FeatureCollection),
  }

// all API Fields
  pub struct Args {
      // The instance or area to lookup in the db to get geofence/data points
      // defaults to ""
      pub instance: Option<String>,

      // radius of the circle to use in calculations
      // defaults to 70m
      pub radius: Option<f64>,

      // min number of points to use with clustering
      // defaults to 1
      pub min_points: Option<usize>,

      // number of times to run through the clustering optimizations
      // defaults to 1
      pub generations: Option<usize>,

      // number of seconds (s) to run the routing algorithm (longer = better routes)
      // defaults to 1
      pub routing_time: Option<i64>,

      // number of devices - not implemented atm
      // defaults to 1
      pub devices: Option<usize>,

      // Custom list of data points to use in calculations - overrides all else
      // defaults to []
      pub data_points: Option<DataPointsArg>,

      // Custom area to use in the SQL query to get data points
      // defaults to empty FeatureCollection
      pub area: Option<GeoFormats>,

      // Run the fast algorithm or not
      // defaults to true
      pub fast: Option<bool>,

      // Format of how the data should be returned
      // defaults to AreaInput type or SingleArray if AreaInput is None
      pub return_type: Option<String>,

      // Only return stats
      // defaults to false
      pub benchmark_mode: Option<bool>,

      // Only count unique points towards the min_count in each cluster
      // defaults to false
      pub only_unique: Option<bool>,

      // Filter spawnpoints by `last_seen` and pokestops/gyms by `updated`
      // defaults to 0
      pub last_seen: Option<i64>,

      // Auto save the results to the scanner database
      // defaults to false
      pub save_to_db: Option<bool>,

      // Number of points to split by when routing
      // Lower = better local routing but may have longer stretches that join the smaller routes
      // defaults to 250
      pub route_chunk_size: Option<usize>
  }

// Benchmark/Stats Struct
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

// Response Struct (what you will receive!)
  pub struct Response {
      pub message: String,
      pub status: String,
      pub status_code: u16,
      pub data: GeoFormats,
      pub stats: Stats,
  }
```

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
  - The geofence saved in the Kōji database in the format specified by `ReturnType` that are related to the specified `Project_Name`

### /api/v1/calc/bootstrap

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance`
  - **Optional**:
    - `radius`
    - `return_type`
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
- **Returns**:
  - Clustered/routing data for the specified area/instance with the specified radius

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
