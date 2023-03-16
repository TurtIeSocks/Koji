import * as React from 'react'
import Map from '@components/Map'
import type { KojiTileServer } from '@assets/types'
import { TileLayer } from 'react-leaflet'

export default function TileServerMap({
  formData,
}: {
  formData: KojiTileServer
}) {
  return (
    <Map
      key={formData.url}
      renderOwnTileLayer
      style={{ width: '100%', height: '50vh' }}
    >
      <TileLayer
        url={
          formData.url ||
          'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png'
        }
      />
    </Map>
  )
}
