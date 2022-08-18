import * as React from 'react'
import { Circle, Polyline } from 'react-leaflet'
import distance from '@turf/distance'

import { useStore } from '@hooks/useStore'
import { getColor } from '@services/utils'
import { getData } from '@services/fetches'
import { COLORS } from '@assets/constants'

export default function Routes() {
  const instance = useStore((s) => s.instance)
  const mode = useStore((s) => s.mode)
  const radius = useStore((s) => s.radius)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const exportSettings = useStore((s) => s.export)
  const setSettings = useStore((s) => s.setSettings)
  const devices = useStore((s) => s.devices)

  React.useEffect(() => {
    if (instance) {
      getData<[number, number][][]>(
        mode === 'bootstrap'
          ? '/api/v1/calc/bootstrap'
          : `/api/v1/calc/${mode}/${category}`,
        { instance, category, radius, generations, devices },
      ).then((route) => {
        let total = 0
        let max = 0
        if (Array.isArray(route)) {
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
          setSettings('export', { ...exportSettings, route, total, max })
        }
      })
    }
  }, [instance, mode, radius, generations, category, devices])

  return showCircles || showLines ? (
    <>
      {exportSettings.route.map((route) => {
        const color =
          mode === 'route'
            ? COLORS[(Math.random() * COLORS.length) | 0]
            : 'blue'
        return route.map((p, j) => {
          if (p.length !== 2 || !p[0] || !p[1]) return null
          const isEnd = j === route.length - 1
          const next = isEnd ? route[0] : route[j + 1]
          const dis = distance(p, next, { units: 'meters' })
          return (
            <React.Fragment key={`${route}-${next}-${isEnd}`}>
              {showCircles && (
                <Circle
                  center={p}
                  radius={radius || 0}
                  color={color}
                  fillColor={color}
                  fillOpacity={0.25}
                  opacity={0.5}
                />
              )}
              {showLines && mode !== 'cluster' && (
                <Polyline
                  positions={[p, next || p]}
                  pathOptions={{ color: getColor(dis), opacity: 80 }}
                />
              )}
            </React.Fragment>
          )
        })
      })}
    </>
  ) : null
}
