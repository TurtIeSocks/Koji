import * as React from 'react'
import { Circle, Polyline } from 'react-leaflet'
import distance from '@turf/distance'

import { useStore } from '@hooks/useStore'
import { getColor } from '@services/utils'
import { getData } from '@services/fetches'

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

  React.useEffect(() => {
    if (instance) {
      getData<[number, number][]>(
        mode === 'bootstrap'
          ? '/api/v1/calc/bootstrap'
          : `/api/v1/calc/${mode}/${category}`,
        { instance, category, radius, generations },
      ).then((route) => {
        let total = 0
        let max = 0
        route.forEach((p, i) => {
          if (p.length !== 2 || !p[0] || !p[1]) return
          const isEnd = i === route.length - 1
          const next = isEnd ? route[0] : route[i + 1]
          const dis = distance(p, next, { units: 'meters' })
          total += dis
          if (dis > max) max = dis
        })
        setSettings('export', { ...exportSettings, route, total, max })
      })
    }
  }, [instance, mode, radius, generations, category])

  return showCircles || showLines ? (
    <>
      {exportSettings.route.map((point, i) => {
        if (point.length !== 2 || !point[0] || !point[1]) return null
        const isEnd = i === exportSettings.route.length - 1
        const next = isEnd
          ? exportSettings.route[0]
          : exportSettings.route[i + 1]
        const dis = distance(point, next, { units: 'meters' })
        return (
          <React.Fragment key={`${point}-${next}-${isEnd}`}>
            {showCircles && (
              <Circle
                center={point}
                radius={radius || 0}
                color="blue"
                fillColor="blue"
                fillOpacity={0.1}
                opacity={0.25}
              />
            )}
            {showLines && mode !== 'cluster' && (
              <Polyline
                positions={[point, next]}
                pathOptions={{ color: getColor(dis), opacity: 80 }}
              />
            )}
          </React.Fragment>
        )
      })}
    </>
  ) : null
}
