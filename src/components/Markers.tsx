import React from 'react'
import { Circle, useMap } from 'react-leaflet'
// import PixiOverlay from 'react-leaflet-pixi-overlay'

import { useStore } from '@hooks/useStore'
import { getSpecificStops } from '@services/utils'
import { PixiMarker } from '@assets/types'
// import usePixi from '@hooks/usePixi'

const ICON_HASH = {
  pokestop:
    '<svg xmlns="http://www.w3.org/2000/svg" fill="green" width="15" height="15" viewBox="0 0 24 24"><circle cx="10" cy="10" r="10" /></svg>',
  gym: '<svg xmlns="http://www.w3.org/2000/svg" fill="maroon" width="20" height="20" viewBox="0 0 24 24"><circle cx="10" cy="10" r="10" /></svg>',
  spawnpoint_true:
    '<svg xmlns="http://www.w3.org/2000/svg" fill="deeppink" width="10" height="10" viewBox="0 0 24 24"><circle cx="10" cy="10" r="10" /></svg>',
  spawnpoint_false:
    '<svg xmlns="http://www.w3.org/2000/svg" fill="dodgerblue" width="10" height="10" viewBox="0 0 24 24"><circle cx="10" cy="10" r="10" /></svg>',
}

export default function Markers() {
  const setLocation = useStore((s) => s.setLocation)
  const setZoom = useStore((s) => s.setZoom)
  const instanceForm = useStore((s) => s.instanceForm)

  const map = useMap()

  const [markers, setMarkers] = React.useState<PixiMarker[]>([])

  React.useEffect(() => {
    // getMarkers().then((incoming) => {
    //   setMarkers([
    //     // ...incoming.gyms,
    //     ...incoming.pokestops,
    //     // ...incoming.spawnpoints,
    //   ])
    // })
    if (instanceForm.name) {
      getSpecificStops(instanceForm.name).then((incoming) => {
        setMarkers(incoming)
      })
    }
  }, [instanceForm.name])

  const onMove = React.useCallback(() => {
    setLocation(Object.values(map.getCenter()) as [number, number])
    setZoom(map.getZoom())
  }, [map])

  React.useEffect(() => {
    map.on('moveend', onMove)
    return () => {
      map.off('moveend', onMove)
    }
  }, [map, onMove])

  const initialMarkers = React.useMemo(
    () => markers.map((i) => ({ ...i, customIcon: ICON_HASH[i.iconId] })),
    [markers],
  )

  // usePixi({ markers: initialMarkers })
  return (
    <>
      {initialMarkers.map((i) => (
        <Circle
          key={i.id}
          center={i.position}
          radius={5}
          pathOptions={{ fillOpacity: 100, fillColor: 'green', color: 'green' }}
        />
      ))}
    </>
  )
}
