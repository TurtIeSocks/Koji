import { GeoJSON as GeoJSONType } from '@assets/types'
import { useStatic, useStore } from '@hooks/useStore'
import { getColor, getGeojson } from '@services/utils'
import React, { Fragment, useEffect } from 'react'
import { Circle, GeoJSON, GeoJSONProps, Polyline } from 'react-leaflet'

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
      {geojson.features.map((feature, i) => {
        const isEnd = i === geojson.features.length - 1
        const reversed = feature.geometry.coordinates.slice().reverse() as [
          number,
          number,
        ]
        const nextReversed = isEnd
          ? reversed
          : (geojson.features[i + 1].geometry.coordinates.slice().reverse() as [
              number,
              number,
            ])
        const color = getColor(reversed, nextReversed)
        return feature.geometry.type === 'Point' ? (
          <Fragment key={feature.properties.id}>
            <Circle
              center={
                feature.geometry.coordinates.slice().reverse() as [
                  number,
                  number,
                ]
              }
              radius={80}
              color="blue"
              fillColor="blue"
              fillOpacity={0.1}
              opacity={0.25}
            />
            <Polyline
              positions={[
                feature.geometry.coordinates.slice().reverse() as [
                  number,
                  number,
                ],
                (geojson.features[i + 1]?.geometry?.coordinates
                  ?.slice()
                  .reverse() as [number, number]) ||
                  feature.geometry.coordinates.slice().reverse(),
              ]}
              pathOptions={{ color, opacity: 80 }}
            />
          </Fragment>
        ) : null
      })}
    </>
  )
}
