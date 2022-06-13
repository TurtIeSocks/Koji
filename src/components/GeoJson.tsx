import { GeoJSON as GeoJSONType } from '@assets/types'
import { useStatic, useStore } from '@hooks/useStore'
import { getGeojson } from '@services/utils'
import React, { useEffect } from 'react'
import { Circle, GeoJSON, GeoJSONProps } from 'react-leaflet'

export default function GeoJsonComponent() {
  const instanceForm = useStore((s) => s.instanceForm)
  const open = useStatic((s) => s.open)

  const [geojson, setGeojson] = React.useState<GeoJSONType>({
    type: 'FeatureCollection',
    features: [],
  })

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
      <GeoJSON
        key={JSON.stringify(geojson)}
        data={geojson as GeoJSONProps['data']}
      />
      {geojson.features.map((feature) =>
        feature.geometry.type === 'Point' ? (
          <Circle
            key={feature.properties.id}
            center={
              feature.geometry.coordinates.slice().reverse() as [number, number]
            }
            radius={80}
            color="blue"
            fillColor="blue"
            fillOpacity={0.5}
          />
        ) : null,
      )}
    </>
  )
}
