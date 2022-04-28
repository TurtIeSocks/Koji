import React from 'react'
import { MapContainer, TileLayer } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import Markers from './Markers'

export default function App() {
  const location = useStore((s) => s.location)
  const zoom = useStore((s) => s.zoom)

  return (
    <MapContainer center={location} zoom={zoom}>
      <TileLayer
        attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
        url="https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png"
      />
      <Markers />
    </MapContainer>
  )
}
