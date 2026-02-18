import nextra from 'nextra'

export default nextra({
  defaultShowCopyCode: true,
})({
  poweredByHeader: false,
  reactStrictMode: true,
  images: {
    unoptimized: false,
  },
  experimental: {
    webpackBuildWorker: true,
    parallelServerBuildTraces: true,
    parallelServerCompiles: true,
  },
  turbopack: {
    resolveAlias: {
      // Path to your `mdx-components` file with extension
      'next-mdx-import-source-file': './mdx-components.tsx',
    },
  },
})
