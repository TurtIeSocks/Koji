import * as React from 'react'
import { Polygon } from 'react-leaflet'

import { useStatic, useStore } from '@hooks/useStore'
import { getData } from '@services/fetches'

export default function QuestPolygon() {
  const instanceForm = useStore((s) => s.instanceForm)
  const open = useStatic((s) => s.open)
  const [points, setPoints] = React.useState<{ lat: number; lng: number }[]>([])

  React.useEffect(() => {
    if (instanceForm.name && !open) {
      getData<[{ lat: number; lon: number }[]]>(
        '/api/instance/area',
        instanceForm,
      ).then((data) => {
        if (data?.[0]) {
          setPoints(data[0].map((p) => ({ lat: p.lat, lng: p.lon })))
        }
      })
    }
  }, [open, instanceForm.name, instanceForm.radius, instanceForm.generations])

  return <Polygon positions={points} />
}
