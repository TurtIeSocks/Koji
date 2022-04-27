/* eslint-disable no-console */
import express from 'express'
import path from 'path'
import { ApolloServer } from 'apollo-server-express'
import dotenv from 'dotenv'

import typeDefs from './graphql/typeDefs'
import resolvers from './graphql/resolvers'
import context from './graphql/context'

dotenv.config()

const app = express()
const port = process.env.PORT || 3001

app.use(express.static(path.join(__dirname, '../../dist')))

const server = new ApolloServer({
  typeDefs,
  resolvers,
  introspection: process.env.NODE_ENV?.includes('develop'),
  context: ({ req }) => ({ req, ...context }),
  formatError: (e) => {
    console.log(e)
    return e
  },
})

app.get('/', (req, res) => {
  res.sendFile(path.join(path.join(__dirname, '../../dist/index.html')))
})

server.start().then(() =>
  server.applyMiddleware({
    app,
    path: '/graphql',
    bodyParserConfig: { limit: '20mb' },
  }),
)

app.listen(port, () => {
  console.log(`Server now listening on port: ${port}`)
})
