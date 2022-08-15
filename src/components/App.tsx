import * as React from 'react'

import { useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'
import Map from './Map'

const cached: { location: [number, number]; zoom: number } = JSON.parse(
  localStorage.getItem('local') || '{ state: { location: [0, 0], zoom: 18 } }',
).state

export default function App() {
  const { setLocation } = useStore.getState()

  const [tileServer, setTileServer] = React.useState(
    'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
  )
  const [fetched, setFetched] = React.useState<boolean>(false)
  const [initial, setInitial] = React.useState<[number, number]>(
    cached.location,
  )

  React.useEffect(() => {
    getData<[number, number, string]>('/api/config').then((res) => {
      const [lat, lon, tileUrl] = res
      if (cached.location[0] === 0 && cached.location[1] === 0) {
        setInitial([lat, lon])
        setLocation([lat, lon])
      }
      if (tileUrl) {
        setTileServer(tileUrl)
      }
      setFetched(true)
    })
  }, [])

  return fetched ? (
    <Map initial={initial} zoom={cached.zoom} tileServer={tileServer} />
  ) : null
}
