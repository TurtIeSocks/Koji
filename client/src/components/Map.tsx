import * as React from 'react'
import { usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'
import { MapContainer, TileLayer } from 'react-leaflet'
import { ATTRIBUTION } from '@assets/constants'
import { withCartoKey } from '@services/carto'

interface Props {
  children?: React.ReactNode
  forcedLocation?: [number, number]
  forcedZoom?: number
  style?: React.CSSProperties
  zoomControl?: boolean
  renderOwnTileLayer?: boolean
}

const Map = React.forwardRef<L.Map, Props>(
  (
    {
      children,
      forcedLocation,
      forcedZoom,
      style,
      zoomControl,
      renderOwnTileLayer,
    },
    ref,
  ) => {
    const { location, zoom } = usePersist.getState()
    const tileServer = usePersist((s) => s.tileServer)
    const cartoApiKey = useStatic((s) => s.cartoApiKey)
    const url = withCartoKey(tileServer, cartoApiKey)

    return (
      <MapContainer
        key="map"
        ref={ref}
        center={forcedLocation ?? location}
        zoom={forcedZoom ?? zoom}
        zoomControl={zoomControl}
        style={style}
        maxBounds={[
          [-85, -180],
          [85, 180],
        ]}
      >
        {!renderOwnTileLayer && (
          <TileLayer key={url} attribution={ATTRIBUTION} url={url} />
        )}
        {children}
      </MapContainer>
    )
  },
)

Map.displayName = 'Map'

export default Map