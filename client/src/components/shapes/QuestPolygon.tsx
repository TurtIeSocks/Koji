import * as React from 'react'
import { Polygon, Circle } from 'react-leaflet'

import type { Shape } from '@assets/types'
import { useStore } from '@hooks/useStore'
import { useStatic } from '@hooks/useStatic'
import { rdmToShapes, rdmToGeojson } from '@services/utils'

export default function QuestPolygon() {
  const instance = useStore((s) => s.instance)
  const showPolygon = useStore((s) => s.showPolygon)

  const setStatic = useStatic((s) => s.setStatic)
  const instances = useStatic((s) => s.instances)

  const [localState, setLocalState] = React.useState<Shape[]>([])

  React.useEffect(() => {
    setLocalState(rdmToShapes(instance, instances))
    setStatic('geojson', rdmToGeojson(instance, instances))
  }, [instance.length, Object.keys(instances).length])

  return showPolygon ? (
    <>
      {localState.map((feature) => {
        if (feature.type === 'circle') {
          return (
            <Circle
              key={feature.id}
              center={[feature.lat, feature.lng]}
              radius={feature.radius}
            />
          )
        }
        if (feature.type === 'polygon') {
          return <Polygon key={feature.id} positions={feature.positions} />
        }
        return null
      })}
    </>
  ) : null
}
