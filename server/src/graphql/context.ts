import { PrismaClient } from "@prisma/client"

export interface Context {
  prisma: PrismaClient
}

export default {
  prisma: new PrismaClient(),
}
