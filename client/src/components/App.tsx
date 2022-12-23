import * as React from 'react'
import { ThemeProvider } from '@mui/material'

import createTheme from '@assets/theme'
import { Config } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'

import Map from './Map'
import Login from './Login'

function ConfigAuth() {
  const { location, setStore } = useStore.getState()
  const { setStatic } = useStatic.getState()
  const loggedIn = useStatic((s) => s.loggedIn)

  const [fetched, setFetched] = React.useState<boolean>(false)

  React.useEffect(() => {
    getData<Config>('/api/config').then((res) => {
      if (res) {
        if (location[0] === 0 && location[1] === 0) {
          setStore('location', [res.start_lat, res.start_lon])
        }
        setStatic('scannerType', res.scanner_type)
        setStatic('loggedIn', res.logged_in)
        if (res.tile_server) {
          setStatic('tileServer', res.tile_server)
        }
      }
      setFetched(true)
    })
  }, [])

  if (!fetched) return null

  return loggedIn ? <Map /> : <Login />
}

export default function App() {
  const darkMode = useStore((s) => s.darkMode)

  const theme = React.useMemo(
    () => createTheme(darkMode ? 'dark' : 'light'),
    [darkMode],
  )

  return (
    <ThemeProvider theme={theme}>
      <ConfigAuth />
    </ThemeProvider>
  )
}
