import * as React from 'react'
import { Polygon } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'
import { useStatic } from '@hooks/useStatic'

export default function QuestPolygon() {
  const instance = useStore((s) => s.instance)
  const showPolygon = useStore((s) => s.showPolygon)

  const scannerType = useStatic((s) => s.scannerType)

  const [points, setPoints] = React.useState<{ lat: number; lng: number }[]>([])

  React.useEffect(() => {
    if (instance && scannerType === 'rdm') {
      getData<[{ lat: number; lon: number }[]]>('/api/instance/area', {
        instance,
      }).then((data) => {
        if (Array.isArray(data)) {
          setPoints(
            data?.[0] ? data[0].map((p) => ({ lat: p.lat, lng: p.lon })) : [],
          )
        }
      })
    } else {
      setPoints([])
    }
  }, [instance])

  return showPolygon ? <Polygon positions={points} /> : null
}
