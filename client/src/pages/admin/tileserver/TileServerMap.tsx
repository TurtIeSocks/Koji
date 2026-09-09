import * as React from 'react'
import Map from '@components/Map'
import type { KojiTileServer } from '@assets/types'
import { TileLayer } from 'react-leaflet'
import { useStatic } from '@hooks/useStatic'
import { withCartoKey } from '@services/carto'

export default function TileServerMap({
  formData,
}: {
  formData: KojiTileServer
}) {
  const cartoApiKey = useStatic((s) => s.cartoApiKey)
  try {
    return (
      <Map
        key={formData.url}
        renderOwnTileLayer
        style={{ width: '100%', height: '50vh' }}
      >
        <TileLayer
          url={withCartoKey(
            formData.url ||
              'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
            cartoApiKey,
          )}
        />
      </Map>
    )
  } catch (e) {
    return <div>URL is invalid, could not load the preview</div>
  }
}