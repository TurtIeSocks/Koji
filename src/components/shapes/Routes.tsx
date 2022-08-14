import * as React from 'react'
import { Circle, Polyline } from 'react-leaflet'

import { useStatic, useStore } from '@hooks/useStore'
import { getColor } from '@services/utils'
import { getData } from '@services/fetches'

export default function Routes() {
  const instanceForm = useStore((s) => s.instanceForm)
  const open = useStatic((s) => s.open)

  const [points, setPoints] = React.useState<[number, number][]>([])

  React.useEffect(() => {
    getData<[number, number][]>('/api/bootstrap', instanceForm).then((res) =>
      setPoints(res || []),
    )
  }, [open, instanceForm.name, instanceForm.radius])

  return (
    <>
      {points.map((point, i) => {
        if (point.length !== 2) return null
        const isEnd = i === points.length - 1
        const next = isEnd ? points[0] : points[i + 1]
        const color = point && next ? getColor(point, next) : 'black'

        return (
          <React.Fragment key={`${point}-${next}-${isEnd}`}>
            <Circle
              center={point}
              radius={instanceForm.radius}
              color="blue"
              fillColor="blue"
              fillOpacity={0.1}
              opacity={0.25}
            />
            <Polyline
              positions={[point, next]}
              pathOptions={{ color, opacity: 80 }}
            />
          </React.Fragment>
        )
      })}
    </>
  )
}
