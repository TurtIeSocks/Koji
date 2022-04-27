import { gql } from '@apollo/client'

export const instances = gql`
  query GetInstances {
    instances {
      name
      type
      radius
      area
    }
  }
`

export const gyms = gql`
  query GetGyms {
    gyms {
      id
      name
      lat
      lon
    }
  }
`

export const spawnpoints = gql`
  query GetSpawnpoints {
    spawnpoints {
      id
      lat
      lon
      updated
      last_seen
      despawn_sec
    }
  }
`

export const pokestops = gql`
  query GetPokestops {
    pokestops {
      id
      name
      lat
      lon
    }
  }
`
