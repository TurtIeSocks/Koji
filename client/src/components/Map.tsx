import * as React from 'react'
import { usePersist } from '@hooks/usePersist'
import { MapContainer, TileLayer } from 'react-leaflet'
import { useStatic } from '@hooks/useStatic'

interface Props {
  children?: React.ReactNode
  forcedLocation?: [number, number]
  forcedZoom?: number
  style?: React.CSSProperties
}

export default function Map({
  children,
  forcedLocation,
  forcedZoom,
  style,
}: Props) {
  const { location, zoom } = usePersist.getState()
  const tileServer = useStatic((s) => s.tileServer)

  return (
    <MapContainer
      key="map"
      center={forcedLocation ?? location}
      zoom={forcedZoom ?? zoom}
      zoomControl={false}
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
}
