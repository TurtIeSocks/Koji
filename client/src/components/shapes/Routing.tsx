import * as React from 'react'
import { Circle, Polyline, Popup } from 'react-leaflet'
import distance from '@turf/distance'
import geohash from 'ngeohash'

import { useStore } from '@hooks/useStore'
import { getColor } from '@services/utils'
import { getLotsOfData } from '@services/fetches'
// import { COLORS } from '@assets/constants'
import { useStatic } from '@hooks/useStatic'
import useDeepCompareEffect from 'use-deep-compare-effect'

export default function Routes() {
  const mode = useStore((s) => s.mode)
  const radius = useStore((s) => s.radius)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const exportSettings = useStore((s) => s.export)
  const setStore = useStore((s) => s.setStore)
  const devices = useStore((s) => s.devices)
  const tab = useStore((s) => s.tab)
  const min_points = useStore((s) => s.min_points)
  const fast = useStore((s) => s.fast)
  const autoMode = useStore((s) => s.autoMode)
  const routing_time = useStore((s) => s.routing_time)
  const only_unique = useStore((s) => s.only_unique)
  const save_to_db = useStore((s) => s.save_to_db)

  const geojson = useStatic((s) => s.geojson)
  const forceRedraw = useStatic((s) => s.forceRedraw)
  const forceFetch = useStatic((s) => s.forceFetch)

  useDeepCompareEffect(() => {
    if (geojson.features.length) {
      setStore('export', { ...exportSettings, route: [[]] })
      getLotsOfData(
        mode === 'bootstrap'
          ? '/api/v1/calc/bootstrap'
          : `/api/v1/calc/${mode}/${category}`,
        {
          category,
          radius,
          generations,
          devices,
          geojson,
          min_points,
          fast,
          routing_time,
          only_unique,
          save_to_db,
        },
      ).then((route) => {
        let total = 0
        let max = 0
        if (Array.isArray(route)) {
          // eslint-disable-next-line no-console
          console.log(route)
          route.forEach((device) => {
            device.forEach((p, i) => {
              if (p.length !== 2 || !p[0] || !p[1]) return
              const isEnd = i === device.length - 1
              const next = isEnd ? device[0] : device[i + 1]
              const dis = distance(p, next, { units: 'meters' })
              total += dis
              if (dis > max) max = dis
            })
          })
          setStore('export', { ...exportSettings, route, total, max })
        }
      })
    }
  }, [
    autoMode
      ? {
          mode,
          radius,
          fast,
          generations,
          min_points,
          category,
          devices,
          geojson,
          tab,
          routing_time,
          only_unique,
          save_to_db,
        }
      : { forceFetch },
  ])

  return showCircles || showLines ? (
    <>
      {exportSettings.route.map((route) => {
        const color =
          // mode === 'route'
          //   ? COLORS[(Math.random() * COLORS.length) | 0]
          //   :
          'green'
        return route.map((p, j) => {
          if (p.length !== 2 || !p[0] || !p[1]) return null
          const isEnd = j === route.length - 1
          const next = isEnd ? route[0] : route[j + 1]
          const dis = distance(p, next, { units: 'meters' })
          return (
            <React.Fragment key={`${next}-${isEnd}-${forceRedraw}`}>
              {showCircles && (
                <Circle
                  center={p}
                  radius={radius || 0}
                  color={j ? color : 'red'}
                  fillColor={j ? color : 'red'}
                  fillOpacity={j ? 0.25 : 0.65}
                  opacity={0.5}
                  snapIgnore
                >
                  {process.env.NODE_ENV === 'development' && (
                    <Popup>
                      <div>
                        Lat: {p[0]}
                        <br />
                        Lng: {p[1]}
                        <br />
                        Hash: {geohash.encode(...p, 9)}
                        <br />
                        Hash: {geohash.encode(...p, 12)}
                      </div>
                    </Popup>
                  )}
                </Circle>
              )}
              {showLines && (
                <Polyline
                  positions={[p, next || p]}
                  pathOptions={{ color: getColor(dis), opacity: 80 }}
                  pmIgnore
                />
              )}
            </React.Fragment>
          )
        })
      })}
    </>
  ) : null
}
