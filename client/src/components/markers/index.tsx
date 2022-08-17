import React from 'react'
import { useMap, Circle } from 'react-leaflet'

import { ICON_SVG, ICON_RADIUS, ICON_COLOR } from '@assets/constants'
import type { PixiMarker } from '@assets/types'
import { useStore } from '@hooks/useStore'
import { getMarkers } from '@services/fetches'
import usePixi from '@hooks/usePixi'

export default function Markers() {
  const location = useStore((s) => s.location)
  const setLocation = useStore((s) => s.setLocation)
  const setZoom = useStore((s) => s.setZoom)
  const data = useStore((s) => s.data)
  const instance = useStore((s) => s.instance)
  const pokestop = useStore((s) => s.pokestop)
  const spawnpoint = useStore((s) => s.spawnpoint)
  const gym = useStore((s) => s.gym)
  const renderer = useStore((s) => s.renderer)

  const map = useMap()

  const [markers, setMarkers] = React.useState<PixiMarker[]>([])

  React.useEffect(() => {
    getMarkers(map, data, instance).then((incoming) => {
      setMarkers([
        ...incoming.gyms,
        ...incoming.pokestops,
        ...incoming.spawnpoints,
      ])
    })
  }, [
    data,
    data === 'area' ? instance : null,
    data === 'bound' ? location : null,
  ])

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
    () =>
      markers
        .filter(
          (x) =>
            ({ v: spawnpoint, u: spawnpoint, g: gym, p: pokestop }[x.iconId]),
        )
        .map((i) => ({ ...i, customIcon: ICON_SVG[i.iconId] })),
    [markers, pokestop, gym, spawnpoint],
  )
  usePixi(renderer === 'performance' ? initialMarkers : [])

  return renderer === 'performance' ? null : (
    <>
      {initialMarkers.map((i) => (
        <Circle
          key={`${i.id}-${i.iconId}`}
          center={i.position}
          radius={ICON_RADIUS[i.iconId]}
          pathOptions={{
            fillOpacity: 100,
            fillColor: ICON_COLOR[i.iconId],
            color: ICON_COLOR[i.iconId],
          }}
        />
      ))}
    </>
  )
}
