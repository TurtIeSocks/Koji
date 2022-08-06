import React from 'react'
// import 'leaflet.locatecontrol'
import * as L from 'leaflet'

interface Props {
  map: L.Map
}

export default function Locate({ map }: Props) {
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
