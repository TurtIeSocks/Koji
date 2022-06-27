/* eslint-disable import/no-extraneous-dependencies */
import { resolve } from 'path'

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import checker from 'vite-plugin-checker'

export default defineConfig(({ mode }) => ({
  plugins: [
    react({
      jsxRuntime: 'classic',
    }),
    checker({
      typescript: true,
      eslint: {
        lintCommand: 'eslint "./src/**/*.{ts,tsx}"',
      },
    }),
  ],
  publicDir: 'public',
  resolve: {
    alias: {
      '@components': resolve(__dirname, './src/components'),
      '@assets': resolve(__dirname, './src/assets'),
      '@hooks': resolve(__dirname, './src/hooks'),
      '@services': resolve(__dirname, './src/services'),
    },
  },
  esbuild: {
    legalComments: 'none',
  },
  build: {
    target: ['safari11.1', 'chrome64', 'firefox66', 'edge88'],
    outDir: resolve(__dirname, './dist'),
    sourcemap: mode === 'development',
    minify: mode === 'development' ? false : 'esbuild',
    input: {
      main: resolve(__dirname, 'index.html'),
    },
    assetsDir: '',
    emptyOutDir: true,
  },
  server: {
    host: '0.0.0.0',
    port: 8081,
    fs: {
      strict: false,
    },
    proxy: {
      '/api': {
        target: `http://localhost:8080`,
        changeOrigin: true,
        secure: false,
      },
    },
  },
}))
