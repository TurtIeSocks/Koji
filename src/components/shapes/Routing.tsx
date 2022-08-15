import * as React from 'react'
import { Circle, Polyline } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { getColor } from '@services/utils'
import { getData } from '@services/fetches'

export default function Routes() {
  const apiSettings = useStore((s) => s.apiSettings)

  const [points, setPoints] = React.useState<[number, number][]>([])

  React.useEffect(() => {
    if (apiSettings.instance) {
      getData<[number, number][]>(
        apiSettings.mode === 'bootstrap'
          ? 'api/bootstrap'
          : '/api/pokestop/route',
        apiSettings,
      ).then((res) => setPoints(Array.isArray(res) ? res : []))
    }
  }, [apiSettings])

  return (
    <>
      {points.map((point, i) => {
        if (point.length !== 2 || !point[0] || !point[1]) return null
        const isEnd = i === points.length - 1
        const next = isEnd ? points[0] : points[i + 1]
        const color = point && next ? getColor(point, next) : 'black'
        return (
          <React.Fragment key={`${point}-${next}-${isEnd}`}>
            <Circle
              center={point}
              radius={apiSettings.radius}
              color="blue"
              fillColor="blue"
              fillOpacity={0.1}
              opacity={0.25}
            />
            {apiSettings.mode !== 'cluster' && (
              <Polyline
                positions={[point, next]}
                pathOptions={{ color, opacity: 80 }}
              />
            )}
          </React.Fragment>
        )
      })}
    </>
  )
}
