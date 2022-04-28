/* eslint-disable no-console */
import express from 'express'
import path from 'path'
import compression from 'compression'
import dotenv from 'dotenv'
import { PrismaClient } from "@prisma/client"

dotenv.config()

const app = express()
const prisma = new PrismaClient()
const port = process.env.PORT || 3001

app.use(compression())

app.use(express.static(path.join(__dirname, '../../dist')))

app.get('/', (req, res) => {
  res.sendFile(path.join(path.join(__dirname, '../../dist/index.html')))
})

app.get('/api', async (req, res) => {
  const spawnpoints = prisma.spawnpoint
    .findMany()
    .then((r) => r.map((i) => ({ ...i, id: i.id.toString() })))
  const pokestops = prisma.pokestop.findMany({
    select: {
      id: true,
      lat: true,
      lon: true,
      updated: true,
    },
    where: { deleted: 0 },
  })
  const gyms = prisma.gym.findMany({
    select: {
      id: true,
      lat: true,
      lon: true,
      updated: true,
    },
    where: { deleted: 0 },
  })
  const results = await Promise.all([spawnpoints, pokestops, gyms])
  res.status(200).json({ spawnpoints: results[0], pokestops: results[1], gyms: results[2] })
})

app.listen(port, () => {
  console.log(`Server now listening on port: ${port}`)
})
