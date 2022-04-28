import { spawnpoint, pokestop, gym } from '@prisma/client'

export interface Data {
  gyms: gym[]
  pokestops: pokestop[]
  spawnpoints: spawnpoint[]
}
