import React from 'react'
import { useMap } from 'react-leaflet'

import { useStore } from '@hooks/useStore'

import Tools from './Tools'
import Spawnpoints from './Spawnpoints'

export default function Movement() {
  const setLocation = useStore((s) => s.setLocation)
  const setZoom = useStore((s) => s.setZoom)
  const map = useMap()

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
    <>
      <Spawnpoints />
      <Tools />
    </>
  )
}
