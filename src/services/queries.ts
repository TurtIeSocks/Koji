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
  query GetGyms($bounds: Bounds!) {
    gyms(bounds: $bounds) {
      id
      name
      lat
      lon
    }
  }
`

export const spawnpoints = gql`
  query GetSpawnpoints($bounds: Bounds!) {
    spawnpoints(bounds: $bounds) {
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
  query GetPokestops($bounds: Bounds!) {
    pokestops(bounds: $bounds) {
      id
      name
      lat
      lon
    }
  }
`
