import React from 'react'
import { MapContainer, TileLayer } from 'react-leaflet'
import { ApolloProvider } from '@apollo/client'
import client from '@services/Apollo'

import { useStore } from '@hooks/useStore'
import Movement from './OnMove'

export default function App() {
  const location = useStore((s) => s.location)
  const zoom = useStore((s) => s.zoom)

  return (
    <ApolloProvider client={client}>
      <MapContainer
        center={location}
        zoom={zoom}
      >
        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
          url="https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png"
        />
        <Movement />
      </MapContainer>
    </ApolloProvider>
  )
}
