import * as React from 'react'
import * as L from 'leaflet'

import { useStatic } from '@hooks/useStore'
import { getButtonHtml } from '@services/utils'

export default function useButtons(map: L.Map) {
  const setOpen = useStatic((s) => s.setOpen)

  const [circleLayer] = React.useState(new L.FeatureGroup())
  const [polygonLayer] = React.useState(new L.FeatureGroup())

  const InstanceSelect = L.Control.extend({
    options: {
      position: 'topright',
    },
    onAdd: () => {
      const container = L.DomUtil.create(
        'div',
        'leaflet-bar leaflet-control leaflet-control-custom',
      )
      container.innerHTML = getButtonHtml('route', 'search')

      container.onclick = () => {
        setOpen('instance')
      }
      return container
    },
  })

  React.useEffect(() => {
    polygonLayer.addTo(map)
    circleLayer.addTo(map)
    map.addControl(new InstanceSelect())
  }, [])
}
