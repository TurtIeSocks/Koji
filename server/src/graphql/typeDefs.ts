import { gql } from 'apollo-server-express'

export default gql`
  scalar JSON
  scalar BigInt

  input Bounds {
    minLat: Float!
    maxLat: Float!
    minLon: Float!
    maxLon: Float!
  }

  type Poi {
    id: ID!
    name: String
    lat: Float
    lon: Float
  }
  
  type Instance {
    name: String!
    type: String!
    max_level: Int
    min_level: Int
    timezone_offset: Int
    radius: Int
    area: JSON
  }

  type Spawnpoint {
    id: BigInt
    lat: Float
    lon: Float
    updated: Int
    last_seen: Int
    despawn_sec: Int
  }

  type Query {
    gyms(bounds: Bounds): [Poi]
    instances: [Instance]
    pokestops(bounds: Bounds): [Poi]
    spawnpoints(bounds: Bounds): [Spawnpoint]
  }
`