import React, { useEffect } from 'react'
import { Circle, useMap } from 'react-leaflet'
import geohash from 'ngeohash'
import { shallow } from 'zustand/shallow'

import { ICON_RADIUS, ICON_COLOR } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'
import usePixi from '@hooks/usePixi'
import { useStatic } from '@hooks/useStatic'
import useDeepCompareEffect from 'use-deep-compare-effect'
import { getMarkers } from '@services/fetches'
import { Category, PixiMarker } from '@assets/types'
import { getDataPointColor } from '@services/utils'

import StyledPopup from '../popups/Styled'
import { GeohashMarker } from './Geohash'

const DEBUG_HASHES: string[] = []

export default function Markers({ category }: { category: Category }) {
  const enabled = usePersist((s) => s[category], shallow)
  const nativeLeaflet = usePersist((s) => s.nativeLeaflet)
  const data = usePersist((s) => s.data)
  const last_seen = usePersist((s) => s.last_seen)
  const pokestopRange = usePersist((s) => s.pokestopRange)
  const tth = usePersist((s) => s.tth)
  const colorByGeoHash = usePersist((s) => s.colorByGeohash)
  const geohashPrecision = usePersist((s) => s.geohashPrecision)

  const updateButton = useStatic((s) => s.updateButton)
  const bounds = useStatic((s) => s.bounds)
  const geojson = useStatic((s) => s.geojson)
  const showCenterCircle = useMap().getZoom() > 15

  const [markers, setMarkers] = React.useState<PixiMarker[]>([])
  const [focused, setFocused] = React.useState(true)

  usePixi(
    nativeLeaflet && process.env.NODE_ENV === 'development'
      ? []
      : category === 'pokestop' && pokestopRange
      ? markers.flatMap((x) => [
          x,
          {
            i: x.i.replace('p', 'r') as PixiMarker['i'],
            p: x.p,
          },
        ])
      : markers,
  )

  useDeepCompareEffect(() => {
    if (focused && !updateButton) {
      const controller = new AbortController()
      const filtered = geojson.features.filter((feature) =>
        feature.geometry.type.includes('Polygon'),
      )
      if (enabled && (data === 'area' ? filtered.length : true)) {
        getMarkers(controller.signal, category, tth).then((res) => {
          if (res.length && res.length !== markers.length) {
            setMarkers(res)
          }
        })
      } else {
        setMarkers([])
      }
      return () => controller.abort()
    }
  }, [
    data,
    data === 'area' ? geojson : {},
    data === 'bound' ? bounds : {},
    enabled,
    last_seen,
    focused,
    pokestopRange,
    tth,
  ])

  const memoSetFocused = React.useCallback(
    () => setFocused(document.hasFocus()),
    [],
  )

  useEffect(() => {
    window.addEventListener('focus', memoSetFocused)
    window.addEventListener('blur', memoSetFocused)
    return () => {
      window.removeEventListener('focus', memoSetFocused)
      window.removeEventListener('blur', memoSetFocused)
    }
  }, [memoSetFocused])

  return (
    <>
      {DEBUG_HASHES.map((hash) => (
        <GeohashMarker key={hash} hash={hash} />
      ))}
      {nativeLeaflet ? (
        <>
          {markers.map((i) => {
            const uniqueHash = geohash.encode(...i.p, 12)
            const groupHash = geohash.encode(...i.p, geohashPrecision)
            return (
              <React.Fragment
                key={`${uniqueHash}${geohashPrecision}${colorByGeoHash}`}
              >
                {pokestopRange && (
                  <Circle
                    center={i.p}
                    radius={70}
                    opacity={0.2}
                    fillOpacity={0.2}
                    fillColor="darkgreen"
                    color="green"
                    pane="dev_markers"
                    pmIgnore
                    snapIgnore
                  />
                )}
                <Circle
                  center={i.p}
                  radius={ICON_RADIUS[i.i[0]]}
                  fillOpacity={0.8}
                  opacity={0.8}
                  fillColor={
                    colorByGeoHash
                      ? getDataPointColor(groupHash)
                      : ICON_COLOR[i.i[0]]
                  }
                  color="black"
                  pane="dev_markers"
                  pmIgnore
                  snapIgnore
                >
                  <StyledPopup>
                    <div>
                      Lat: {i.p[0]}
                      <br />
                      Lng: {i.p[1]}
                      <br />
                      Hash: {groupHash}
                      <br />
                      Hash: {uniqueHash}
                    </div>
                  </StyledPopup>
                </Circle>
                {showCenterCircle && (
                  <Circle
                    center={i.p}
                    radius={1}
                    pathOptions={{
                      fillColor: 'black',
                      color: 'black',
                    }}
                    pane="dev_markers"
                    pmIgnore
                    snapIgnore
                  />
                )}
              </React.Fragment>
            )
          })}
        </>
      ) : null}
    </>
  )
}
