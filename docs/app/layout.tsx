import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Head } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import 'nextra-theme-docs/style.css'

export const metadata = {
  title: {
    default: 'Kōji',
    template: '%s',
  },
  description: 'Rust based geofence editor, manager, and distributor',
  openGraph: {
    type: 'website',
    title: 'Kōji',
    description: 'Rust based geofence editor, manager, and distributor',
    images: [
      'https://github.com/TurtIeSocks/Koji/blob/main/client/public/favicon.png',
    ],
  },
}

export const viewport = {
  width: 'device-width',
  initialScale: 1,
}

const navbar = (
  <Navbar
    logo={<span>Kōji</span>}
    projectLink="https://github.com/TurtIeSocks/Koji"
    chatLink="https://discord.gg/EYYsKPVawn"
  />
)
const footer = (
  <Footer>
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
  </Footer>
)

export default async function RootLayout({ children }) {
  return (
    <html
      // Not required, but good for SEO
      lang="en"
      // Required to be set
      dir="ltr"
      // Suggested by `next-themes` package https://github.com/pacocoursey/next-themes#with-app
      suppressHydrationWarning
    >
      <Head>
        {/* Your additional tags should be passed as `children` of `<Head>` element */}
      </Head>
      <body>
        <Layout
          navbar={navbar}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/TurtIeSocks/Koji/edit/main/docs"
          footer={footer}
          // feedback={{ content: null }}
          // ... Your additional layout options
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
