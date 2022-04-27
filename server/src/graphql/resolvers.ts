import { GraphQLJSON, GraphQLBigInt } from 'graphql-scalars'

import { Context } from './context'

export default {
  JSON: GraphQLJSON,
  BigInt: GraphQLBigInt,
  Query: {
    gyms: async (_parent: unknown, _args: null, { prisma }: Context) => {
      const results = await prisma.gym.findMany()
      return results
    },
    instances: async (_parent: unknown, _args: null, { prisma }: Context) => {
      const results = await prisma.instance.findMany()
      return results.map((result) => ({
        ...result,
        ...JSON.parse(result.data),
      }))
    },
    pokestops: async (_parent: unknown, _args: null, { prisma }: Context) => {
      const results = await prisma.pokestop.findMany()
      return results
    },
    spawnpoints: async (_parent: unknown, _args: null, { prisma }: Context) => {
      const results = await prisma.spawnpoint.findMany()
      return results
    },
  },
}
