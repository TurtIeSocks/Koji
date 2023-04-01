import React from 'react'
import { DocsThemeConfig } from 'nextra-theme-docs'

const config: DocsThemeConfig = {
  logo: <span>Kōji</span>,
  head: (
    <>
      <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      <meta property="og:title" content="Kōji" />
      <meta property="og:type" content="website" />
      <meta
        property="og:description"
        content="Rust based geofence editor, manager, and distributor"
      />
      <meta
        property="og:image"
        content="https://github.com/TurtIeSocks/Koji/blob/main/client/public/favicon.png"
      />
    </>
  ),
  project: {
    link: 'https://github.com/TurtIeSocks/Koji',
  },
  chat: {
    link: 'https://discord.gg/EYYsKPVawn',
  },
  feedback: {
    content: null,
  },
  docsRepositoryBase:
    'https://github.com/TurtIeSocks/Koji/tree/main/docs/pages',
  footer: {
    text: 'Kōji Docs',
  },
  useNextSeoProps() {
    return {
      titleTemplate: '%s',
    }
  },
}

export default config
