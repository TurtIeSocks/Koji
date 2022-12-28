import * as React from 'react'

import { Config } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { getData } from '@services/fetches'

import Login from '@pages/home/Login'
import Splash from './Splash'

export default function Home() {
  const { location, setStore } = usePersist.getState()
  const { setStatic } = useStatic.getState()
  const loggedIn = useStatic((s) => s.loggedIn)

  const [fetched, setFetched] = React.useState<boolean>(false)

  React.useEffect(() => {
    getData<Config>('/config').then((res) => {
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

  return loggedIn ? <Splash /> : <Login />
}
