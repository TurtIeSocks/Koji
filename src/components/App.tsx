import React from 'react'
import { ApolloProvider } from '@apollo/client'
import { MapContainer, TileLayer } from 'react-leaflet'

import client from '@services/Apollo'
import Tools from './Tools'

export default function App() {
  return (
    <ApolloProvider client={client}>
      <MapContainer
        center={[+inject.START_LAT || 0, +inject.START_LON || 0]}
        zoom={13}
      >
        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
          url="https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png"
        />
        <Tools />

      </MapContainer>
    </ApolloProvider>
  )
}
