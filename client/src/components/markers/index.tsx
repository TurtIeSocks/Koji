import React from 'react'
import { useMap, Circle, Popup } from 'react-leaflet'
import geohash from 'ngeohash'

import { ICON_RADIUS, ICON_COLOR } from '@assets/constants'
import { useStore } from '@hooks/useStore'
import { getMarkers } from '@services/fetches'
import usePixi from '@hooks/usePixi'
import { useStatic } from '@hooks/useStatic'
import useDeepCompareEffect from 'use-deep-compare-effect'

export default function Markers() {
  const location = useStore((s) => s.location)
  const data = useStore((s) => s.data)
  const pokestop = useStore((s) => s.pokestop)
  const spawnpoint = useStore((s) => s.spawnpoint)
  const gym = useStore((s) => s.gym)
  const nativeLeaflet = useStore((s) => s.nativeLeaflet)
  const last_seen = useStore((s) => s.last_seen)
  const setStore = useStore((s) => s.setStore)

  const geojson = useStatic((s) => s.geojson)
  const pokestops = useStatic((s) => s.pokestops)
  const spawnpoints = useStatic((s) => s.spawnpoints)
  const gyms = useStatic((s) => s.gyms)
  const setStatic = useStatic((s) => s.setStatic)

  const map = useMap()

  useDeepCompareEffect(() => {
    getMarkers(map, data, geojson, pokestop, spawnpoint, gym, last_seen).then(
      (incoming) => {
        setStatic('pokestops', incoming.pokestops)
        setStatic('spawnpoints', incoming.spawnpoints)
        setStatic('gyms', incoming.gyms)
      },
    )
  }, [
    data,
    data === 'area' ? geojson : null,
    data === 'bound' ? location : null,
    pokestop,
    spawnpoint,
    gym,
    last_seen,
  ])

  const onMove = React.useCallback(() => {
    setStore('location', Object.values(map.getCenter()) as [number, number])
    setStore('zoom', map.getZoom())
  }, [map])

  React.useEffect(() => {
    map.on('moveend', onMove)
    return () => {
      map.off('moveend', onMove)
    }
  }, [map, onMove])

  const initialMarkers = React.useMemo(
    () => [
      ...(pokestop ? pokestops : []),
      ...(spawnpoint ? spawnpoints : []),
      ...(gym ? gyms : []),
    ],
    [
      pokestops.length,
      gyms.length,
      spawnpoints.length,
      pokestop,
      gym,
      spawnpoint,
    ],
  )

  usePixi(nativeLeaflet ? [] : initialMarkers)

  return nativeLeaflet ? (
    <>
      {initialMarkers.map((i) => (
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
