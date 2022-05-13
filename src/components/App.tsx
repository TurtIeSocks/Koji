import React from 'react'
import { MapContainer, TileLayer } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { getConfig } from '@services/utils'

import Markers from './Markers'

const cached: { location: [number, number], zoom: number } = JSON.parse(
  localStorage.getItem('local') || '{ state: { location: [0, 0], 18 } }',
).state

export default function App() {
  const setLocation = useStore((s) => s.setLocation)
  const [tileServer, setTileServer] = React.useState('https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png')
  const [fetched, setFetched] = React.useState<boolean>(false)
  const [initial, setInitial] = React.useState<[number, number]>(cached.location)

  React.useEffect(() => {
    getConfig().then((res) => {
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

  return (
    <MapContainer key={initial.join('')} center={initial} zoom={cached.zoom}>
      <TileLayer
        key={tileServer}
        attribution="RDM Tools 2.0 - TurtleSocks"
        url={tileServer}
      />
      {fetched && <Markers />}
    </MapContainer>
  )
}
