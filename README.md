# Kōji

<!-- [![GitHub Release](https://img.shields.io/github/release/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/releases/)
[![GitHub Contributors](https://img.shields.io/github/contributors/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/graphs/contributors/) -->

[![Discord](https://img.shields.io/discord/907337201044582452.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/EYYsKPVawn)

## Features

- Geofence manager, editor, and distribution tool
- API based clustering for automatic route making
- Fast route solver for quest scanning
- Geofence conversions into various formats

## Compatibility

- [RealDeviceMap (RDM)](https://github.com/realdevicemap/realdevicemap)

## Installation

### Standard

1. Clone this repo:

```bash
git clone https://github.com/TurtIeSocks/Koji.git
```

2. [Install NodeJS](https://nodejs.dev/en/learn/how-to-install-nodejs/)
3. [Install Rust](https://www.rust-lang.org/tools/install)
4. Copy the env file:

```bash
cd server && cp .env.example .env
```

5. Edit the env file: `nano .env`
   - Set the `DATABASE_URL` to your RDM database url
   - Temporarily set `NODE_ENV` to `development`
   - Set `PORT` to whatever you want
   - Set `START_LAT` and `START_LON` to wherever you want the map to start
6. Compile the client:

```bash
cd ../client && yarn install && yarn build
```

7. Compile the server:

```bash
cd ../server && cargo run -r
# you might have to also install pkg-config (`apt install pkg-config`)
```

### Docker (Recommended)

1. Get the docker-compose.yml example file:

```bash
curl https://raw.githubusercontent.com/TurtIeSocks/Koji/main/docker-compose.example.yml > docker-compose.yml
```

2. `nano docker-compose.yml`
3. Set the same env variables as above
4. `docker-compose pull`
5. `docker-compose up -d`

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
cargo watch -x run
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
// Accepted Area Inputs:
  pub enum AreaInput {
      Text(String),
      // can be either:
        // lat,lon\nlat,lon
        // or lat lon,lat lon
      SingleArray(Vec<[f64; 2]>),
      MultiArray(Vec<Vec<[f64; 2]>>),
      SingleStruct(Vec<{ lat: f64, lon: f64}>),
      MultiStruct(Vec<Vec<{ lat: f64, lon: f64}>>),
      Feature(Feature), // GeoJSON
      FeatureCollection(FeatureCollection), // GeoJSON
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
    FeatureCollection,
}
// all API Fields
  pub struct API {
      pub instance: Option<String>,
      // defaults to ""
      pub radius: Option<f64>,
      // defaults to 70m
      pub min_points: Option<usize>,
      // defaults to 1
      pub generations: Option<usize>,
      // defaults to 1
      pub devices: Option<usize>,
      // defaults to 1
      pub data_points: Option<Vec<{ lat: f64, lon: f64}>>,
      // defaults to []
      pub area: Option<AreaInput>,
      // defaults to empty FeatureCollection
      pub fast: Option<bool>,
      // defaults to true
      pub return_type: Option<String>,
      // defaults to AreaInput type or SingleArray if AreaInput is None
  }
```

### /api/v1/calc/bootstrap

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance`
  - **Optional**:
    - `radius`
    - `return_type`

### /api/v1/calc/cluster

### /api/v1/calc/route

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance`
  - **Optional**:
    - `radius`
    - `return_type`
    - `min_points`
    - `generations`
    - `devices`
    - `data_points`
    - `fast`

### /api/v1/convert/data

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area`
  - **Optional**:
    - `return_type`
