import * as React from 'react'
import { Polygon } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'

export default function QuestPolygon() {
  const instanceForm = useStore((s) => s.apiSettings)
  const [points, setPoints] = React.useState<{ lat: number; lng: number }[]>([])

  React.useEffect(() => {
    if (instanceForm.instance) {
      getData<[{ lat: number; lon: number }[]]>(
        '/api/instance/area',
        instanceForm,
      ).then((data) => {
        if (data?.[0]) {
          setPoints(data[0].map((p) => ({ lat: p.lat, lng: p.lon })))
        }
      })
    }
  }, [instanceForm.instance])

  return <Polygon positions={points} />
}
