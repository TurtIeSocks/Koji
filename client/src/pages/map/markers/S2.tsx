import * as React from 'react'
import { GeoJSON, useMap } from 'react-leaflet'
import { Feature, KojiResponse } from '@assets/types'
import { fetchWrapper } from '@services/fetches'
import { getMapBounds } from '@services/utils'
import { usePersist } from '@hooks/usePersist'

export function S2({ level }: { level: number }) {
  const location = usePersist((s) => s.location)
  const map = useMap()

  const [data, setData] = React.useState<Feature | null>(null)

  React.useEffect(() => {
    fetchWrapper<KojiResponse<Feature>>(`/api/v1/s2/${level}`, {
      method: 'POST',
      body: JSON.stringify({
        ...getMapBounds(map),
      }),
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((res) => res && setData(res.data))
  }, [location])

  return data && <GeoJSON key={JSON.stringify(data)} data={data} />
}
