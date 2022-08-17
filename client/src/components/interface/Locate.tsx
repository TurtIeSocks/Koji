import React from 'react'
import 'leaflet.locatecontrol'
import * as L from 'leaflet'
import { useMap } from 'react-leaflet'

export default function Locate() {
  const map = useMap()

  const [lc] = React.useState(
    L.control.locate({
      position: 'bottomright',
      keepCurrentZoomLevel: true,
      setView: 'untilPan',
    }),
  )

  React.useEffect(() => {
    lc.addTo(map)
  }, [])

  return null
}
