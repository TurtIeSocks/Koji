import * as React from 'react'
import { Polygon } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'

export default function QuestPolygon() {
  const instance = useStore((s) => s.instance)
  const showPolygon = useStore((s) => s.showPolygon)

  const [points, setPoints] = React.useState<{ lat: number; lng: number }[]>([])

  React.useEffect(() => {
    if (instance) {
      getData<[{ lat: number; lon: number }[]]>('/api/instance/area', {
        instance,
      }).then((data) => {
        if (data?.[0]) {
          setPoints(data[0].map((p) => ({ lat: p.lat, lng: p.lon })))
        }
      })
    }
  }, [instance])

  return showPolygon ? <Polygon positions={points} /> : null
}
