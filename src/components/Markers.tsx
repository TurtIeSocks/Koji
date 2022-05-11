import React from 'react'
import { useMap } from 'react-leaflet'
import PixiOverlay from 'react-leaflet-pixi-overlay'

import { useStore } from '@hooks/useStore'
import { getMarkers, getSpawnpoints } from '@services/utils'
import { PixiMarker } from '@assets/types'

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
  const map = useMap()
  const [markers, setMarkers] = React.useState<PixiMarker[]>([])
  const [points, setPoints] = React.useState<PixiMarker[]>([])

  React.useEffect(() => {
    getMarkers(map).then((incoming) => {
      if (inject.ALL_SPAWNPOINTS) {
        setMarkers([
          ...incoming.gyms,
          ...incoming.pokestops,
          ...incoming.spawnpoints,
        ])
      } else {
        setMarkers([...incoming.gyms, ...incoming.pokestops])
        setPoints(incoming.spawnpoints)
      }
    })
  }, [])

  const onMove = React.useCallback(() => {
    setLocation(Object.values(map.getCenter()) as [number, number])
    setZoom(map.getZoom())
    if (!inject.ALL_SPAWNPOINTS) {
      getSpawnpoints(map).then((incoming) => {
        setPoints(incoming)
      })
    }
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
  const spawnMarkers = React.useMemo(
    () => points.map((i) => ({ ...i, customIcon: ICON_HASH[i.iconId] })),
    [points],
  )

  return inject.ALL_SPAWNPOINTS ? (
    <PixiOverlay markers={initialMarkers} />
  ) : (
    <PixiOverlay markers={[...initialMarkers, ...spawnMarkers]} />
  )
}
