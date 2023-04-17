import * as React from 'react'
import { usePersist } from '@hooks/usePersist'
import { MapContainer, TileLayer } from 'react-leaflet'
import { ATTRIBUTION } from '@assets/constants'

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

    return (
      <MapContainer
        key="map"
        ref={ref}
        center={forcedLocation ?? location}
        zoom={forcedZoom ?? zoom}
        zoomControl={zoomControl}
        style={style}
        maxBounds={[
          [-90, -210],
          [90, 210],
        ]}
      >
        {!renderOwnTileLayer && (
          <TileLayer
            key={tileServer}
            attribution={ATTRIBUTION}
            url={tileServer}
          />
        )}
        {children}
      </MapContainer>
    )
  },
)

Map.displayName = 'Map'

export default Map
