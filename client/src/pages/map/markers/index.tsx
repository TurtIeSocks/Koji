import React, { useEffect } from 'react'
import { Circle, Popup, useMap } from 'react-leaflet'
import geohash from 'ngeohash'

import { ICON_RADIUS, ICON_COLOR } from '@assets/constants'
import { UsePersist, usePersist } from '@hooks/usePersist'
import usePixi from '@hooks/usePixi'
import { useStatic } from '@hooks/useStatic'
import useDeepCompareEffect from 'use-deep-compare-effect'
import { getMarkers } from '@services/fetches'
import { getMapBounds } from '@services/utils'
import { PixiMarker } from '@assets/types'

export default function Markers({
  category,
}: {
  category: UsePersist['category']
}) {
  const enabled = usePersist((s) => s[category])
  const nativeLeaflet = usePersist((s) => s.nativeLeaflet)
  const location = usePersist((s) => s.location)
  const data = usePersist((s) => s.data)
  const last_seen = usePersist((s) => s.last_seen)

  const geojson = useStatic((s) => s.geojson)

  const [markers, setMarkers] = React.useState<PixiMarker[]>([])
  const [focused, setFocused] = React.useState(true)

  const map = useMap()

  const memoSetFocused = React.useCallback(
    () => setFocused(document.hasFocus()),
    [],
  )

  usePixi(nativeLeaflet ? [] : markers)

  useDeepCompareEffect(() => {
    if (focused) {
      const controller = new AbortController()
      const filtered = geojson.features.filter((feature) =>
        feature.geometry.type.includes('Polygon'),
      )
      if (enabled && (data === 'area' ? filtered.length : true)) {
        getMarkers(
          category,
          getMapBounds(map),
          data,
          {
            ...geojson,
            features: filtered,
          },
          last_seen,
          controller.signal,
        ).then((res) => {
          if (res.length && res.length !== markers.length) setMarkers(res)
        })
      } else {
        setMarkers([])
      }
      return () => controller.abort()
    }
  }, [
    data,
    data === 'area' ? geojson : {},
    data === 'bound' ? location : {},
    enabled,
    last_seen,
    focused,
  ])

  useEffect(() => {
    window.addEventListener('focus', memoSetFocused)
    window.addEventListener('blur', memoSetFocused)
    return () => {
      window.removeEventListener('focus', memoSetFocused)
      window.removeEventListener('blur', memoSetFocused)
    }
  }, [memoSetFocused])

  return nativeLeaflet ? (
    <>
      {markers.map((i) => (
        <Circle
          key={i.i}
          center={i.p}
          radius={ICON_RADIUS[i.i[0]]}
          pathOptions={{
            fillOpacity: 0.8,
            opacity: 0.8,
            fillColor: ICON_COLOR[i.i[0]],
            color: 'black',
          }}
        >
          {process.env.NODE_ENV === 'development' && (
            <>
              <Circle
                key={i.i}
                center={i.p}
                radius={1}
                pathOptions={{
                  fillColor: 'black',
                  color: 'black',
                }}
              />
              <Popup>
                <div>
                  Lat: {i.p[0]}
                  <br />
                  Lng: {i.p[1]}
                  <br />
                  Hash: {geohash.encode(...i.p, 9)}
                  <br />
                  Hash: {geohash.encode(...i.p, 12)}
                </div>
              </Popup>
            </>
          )}
        </Circle>
      ))}
    </>
  ) : null
}
