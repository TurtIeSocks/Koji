import React from 'react'
import { DocsThemeConfig } from 'nextra-theme-docs'

const config: DocsThemeConfig = {
  logo: <span>Kōji</span>,
  head: (
    <>
      <link rel="icon" href="/favicon.png" />
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
  docsRepositoryBase: 'https://github.com/TurtIeSocks/Koji/edit/main/docs',
  footer: {
    text: (
      <div
        style={{
          display: 'flex',
          flexDirection: 'row',
          justifyContent: 'space-around',
          width: '100%',
        }}
      >
        <div style={{ flexGrow: 1 }}>Kōji Docs</div>
        <a
          href="https://github.com/sponsors/TurtIeSocks"
          referrerPolicy="no-referrer"
          target="_blank"
          style={{ flexGrow: 0 }}
        >
          Support this Project
        </a>
      </div>
    ),
  },
  useNextSeoProps() {
    return {
      titleTemplate: '%s',
    }
  },
}

export default config
