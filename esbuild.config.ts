/* eslint-disable no-console */
/* eslint-disable import/no-extraneous-dependencies */
import path from 'path'
import fs from 'fs'
import { config } from 'dotenv'
import { build } from 'esbuild'
import { htmlPlugin } from '@craftamap/esbuild-plugin-html'
import { eslintPlugin } from 'esbuild-plugin-eslinter'

const env = fs.existsSync(`${__dirname}/.env`) ? config() : { parsed: {} }
const isDevelopment = Boolean(process.argv.includes('--dev'))
const isRelease = Boolean(process.argv.includes('--release'))

if (fs.existsSync(path.resolve(__dirname, 'dist'))) {
  console.log('[BUILD] Cleaning up old build')
  fs.rm(path.resolve(__dirname, 'dist'), { recursive: true }, (err) => {
    if (err) console.log(err)
  })
}

const plugins = [
  htmlPlugin({
    files: [
      {
        entryPoints: ['src/index.tsx'],
        filename: 'index.html',
        htmlTemplate: fs.readFileSync('./public/index.html').toString(),
        scriptLoading: 'defer',
      },
    ],
  }),
]

if (isDevelopment) {
  plugins.push(eslintPlugin())
}

build({
  entryPoints: ['src/index.tsx'],
  legalComments: 'none',
  bundle: true,
  outdir: 'dist/',
  publicPath: '/',
  entryNames: isDevelopment ? undefined : '[name].[hash]',
  minify: isRelease || !isDevelopment,
  metafile: true,
  logLevel: isDevelopment ? 'info' : 'error',
  target: ['safari11.1', 'chrome64', 'firefox62', 'edge88'],
  watch: isDevelopment
    ? {
        onRebuild(error) {
          if (error) console.error('Recompiling failed:', error)
          else console.log('Recompiled successfully')
        },
      }
    : false,
  sourcemap: isRelease || isDevelopment,
  define: {
    inject: JSON.stringify({ ...env.parsed, DEVELOPMENT: isDevelopment }),
  },
  plugins,
})
  .catch((e) => console.error(e))
  .then(() => console.log('Compiled Successfully'))
