import * as React from 'react'
import { Circle, Polyline } from 'react-leaflet'

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

  const [points, setPoints] = React.useState<[number, number][]>([])

  React.useEffect(() => {
    if (instance) {
      getData<[number, number][]>(
        mode === 'bootstrap'
          ? '/api/v1/calc/bootstrap'
          : `/api/v1/calc/${mode}/${category}`,
        { instance, category, radius, generations },
      ).then((res) => setPoints(Array.isArray(res) ? res : []))
    }
  }, [instance, mode, radius, generations, category])

  return showCircles || showLines ? (
    <>
      {points.map((point, i) => {
        if (point.length !== 2 || !point[0] || !point[1]) return null
        const isEnd = i === points.length - 1
        const next = isEnd ? points[0] : points[i + 1]
        const color = point && next ? getColor(point, next) : 'black'
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
            {showLines && (
              <Polyline
                positions={[point, next]}
                pathOptions={{ color, opacity: 80 }}
              />
            )}
          </React.Fragment>
        )
      })}
    </>
  ) : null
}
