import { GraphQLJSON, GraphQLBigInt } from 'graphql-scalars'

import { Context } from './context'

export interface Bounds {
  bounds: {
    minLat: number
    maxLat: number
    minLon: number
    maxLon: number
  }
}

export default {
  JSON: GraphQLJSON,
  BigInt: GraphQLBigInt,
  Query: {
    gyms: async (_parent: unknown, { bounds }: Bounds, { prisma }: Context) => {
      const results = await prisma.gym.findMany({
        select: {
          id: true,
          lat: true,
          lon: true,
          updated: true,
        },
        where: {
          lat: {
            gt: bounds.minLat,
            lt: bounds.maxLat,
          },
          lon: {
            gt: bounds.minLon,
            lt: bounds.maxLon,
          },
          deleted: 0,
        }
      })
      return results
    },
    instances: async (_parent: unknown, _args: null, { prisma }: Context) => {
      const results = await prisma.instance.findMany()
      return results.map((result) => ({
        ...result,
        ...JSON.parse(result.data),
      }))
    },
    pokestops: async (_parent: unknown, { bounds }: Bounds, { prisma }: Context) => {
      const results = await prisma.pokestop.findMany({
        select: {
          id: true,
          lat: true,
          lon: true,
          updated: true,
        },
        where: {
          lat: {
            gt: bounds.minLat,
            lt: bounds.maxLat,
          },
          lon: {
            gt: bounds.minLon,
            lt: bounds.maxLon,
          },
          deleted: 0,
        }
      })
      return results
    },
    spawnpoints: async (_parent: unknown, { bounds }: Bounds, { prisma }: Context) => {
      const results = await prisma.spawnpoint.findMany({
        where: {
          lat: {
            gt: bounds.minLat,
            lt: bounds.maxLat,
          },
          lon: {
            gt: bounds.minLon,
            lt: bounds.maxLon,
          },
        }
      })
      return results
    },
  },
}
