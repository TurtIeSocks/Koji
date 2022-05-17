import React from 'react'
import 'leaflet.locatecontrol'
import L, { Map } from 'leaflet'

export interface LocateProps {
  map: Map
}

export default function Locate({ map }: LocateProps) {
  const [lc] = React.useState(L.control.locate({
    position: 'bottomright',
    keepCurrentZoomLevel: true,
    setView: 'untilPan',
  }))

  React.useEffect(() => {
    lc.addTo(map)
  })
  return (
    <div />
  )
}