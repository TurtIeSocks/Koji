import { gql } from 'apollo-server-express'

export default gql`
  scalar JSON
  scalar BigInt

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
    gyms: [Poi]
    instances: [Instance]
    pokestops: [Poi]
    spawnpoints: [Spawnpoint]
  }
`