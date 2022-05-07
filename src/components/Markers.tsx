import React from 'react'
import { useMap } from 'react-leaflet'
import PixiOverlay from 'react-leaflet-pixi-overlay'

import { useStore } from '@hooks/useStore'
import { buildMarkers, getData } from '@services/utils'
import { Data } from '@assets/types'

export default function Interface() {
  const setLocation = useStore((s) => s.setLocation)
  const setZoom = useStore((s) => s.setZoom)
  const map = useMap()
  const [data, setData] = React.useState<Data>({
    gyms: [],
    pokestops: [],
    spawnpoints: [],
  })

  React.useEffect(() => {
    getData().then((incoming) => {
      setData(incoming)
    })
  }, [])

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

  return (
    <PixiOverlay markers={React.useMemo(() => buildMarkers(data), [data])} />
  )
}
