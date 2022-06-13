import { useStatic, useStore } from '@hooks/useStore'
import { getColor, getGeojson } from '@services/utils'
import React, { Fragment, useEffect } from 'react'
import { Circle, Polyline } from 'react-leaflet'

export default function GeoJsonComponent() {
  const instanceForm = useStore((s) => s.instanceForm)
  const open = useStatic((s) => s.open)

  const [points, setGeojson] = React.useState<[number, number][]>([])

  useEffect(() => {
    if (
      instanceForm.name &&
      instanceForm.radius &&
      instanceForm.generations &&
      !open
    ) {
      getGeojson(instanceForm).then((res) => setGeojson(res))
    }
  }, [open, instanceForm.name, instanceForm.radius, instanceForm.generations])

  return (
    <>
      {points.map((point, i) => {
        if (point.length !== 2) return null
        const isEnd = i === points.length - 1
        const next = isEnd ? point : points[i + 1]
        const color = point && next ? getColor(point, next) : 'black'

        return (
          <Fragment key={`${point}-${next}-${isEnd}`}>
            <Circle
              center={point}
              radius={80}
              color="blue"
              fillColor="blue"
              fillOpacity={0.1}
              opacity={0.25}
            />
            <Polyline
              positions={[point, next]}
              pathOptions={{ color, opacity: 80 }}
            />
          </Fragment>
        )
      })}
    </>
  )
}
