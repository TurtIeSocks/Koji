import * as React from 'react'

import { Config } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'

import Map from './Map'

export default function App() {
  const { location, setStore } = useStore.getState()
  const { setStatic } = useStatic.getState()

  const [fetched, setFetched] = React.useState<boolean>(false)

  React.useEffect(() => {
    getData<Config>('/api/config').then((res) => {
      if (res) {
        if (location[0] === 0 && location[1] === 0) {
          setStore('location', [res.start_lat, res.start_lon])
        }
        setStatic('scannerType', res.scanner_type)
        if (res.tile_server) {
          setStatic('tileServer', res.tile_server)
        }
      }
      setFetched(true)
    })
  }, [])

  return fetched ? <Map /> : null
}
