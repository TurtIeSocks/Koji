import * as React from 'react'
import { usePersist } from '@hooks/usePersist'
import { MapContainer, TileLayer } from 'react-leaflet'
import { useStatic } from '@hooks/useStatic'

interface Props {
  children?: React.ReactNode
  forcedLocation?: [number, number]
  forcedZoom?: number
  style?: React.CSSProperties
  zoomControl?: boolean
}

const Map = React.forwardRef<L.Map, Props>(
  ({ children, forcedLocation, forcedZoom, style, zoomControl }, ref) => {
    const { location, zoom } = usePersist.getState()
    const tileServer = useStatic((s) => s.tileServer)

    return (
      <MapContainer
        key="map"
        ref={ref}
        center={forcedLocation ?? location}
        zoom={forcedZoom ?? zoom}
        zoomControl={zoomControl}
        style={style}
      >
        <TileLayer
          key={tileServer}
          attribution="<a href='https://github.com/TurtIeSocks/Koji' noreferrer='true' target='_blank'>Kōji - TurtleSocks</a>"
          url={tileServer}
        />
        {children}
      </MapContainer>
    )
  },
)

Map.displayName = 'Map'

export default Map
