import React from 'react'
import { useMap } from 'react-leaflet'

import { ICON_HASH } from '@assets/constants'
import type { PixiMarker } from '@assets/types'
import { useStore } from '@hooks/useStore'
import { getMarkers } from '@services/fetches'
import usePixi from '@hooks/usePixi'

export default function Markers() {
  const setLocation = useStore((s) => s.setLocation)
  const setZoom = useStore((s) => s.setZoom)
  const instanceForm = useStore((s) => s.instanceForm)

  const map = useMap()

  const [markers, setMarkers] = React.useState<PixiMarker[]>([])

  React.useEffect(() => {
    getMarkers().then((incoming) => {
      setMarkers([
        ...incoming.gyms,
        ...incoming.pokestops,
        ...incoming.spawnpoints,
      ])
    })
    // if (instanceForm.name) {
    //   getSpecificStops(instanceForm.name).then((incoming) => {
    //     setMarkers(incoming)
    //   })
    // }
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

  usePixi(initialMarkers)
  return null
  //    (
  //     <>
  //       {/* {initialMarkers.map((i) => (
  //         <Circle
  //           key={i.id}
  //           center={i.position}
  //           radius={5}
  //           pathOptions={{ fillOpacity: 100, fillColor: 'green', color: 'green' }}
  //         />
  //       ))} */}
  //     </>
  //   )
}
