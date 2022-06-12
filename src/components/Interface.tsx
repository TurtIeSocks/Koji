import { GeoJSON as GeoJSONType } from '@assets/types'
import { getData, getGeojson } from '@services/utils'
import React, { useEffect } from 'react'
import {
  useMap,
  ZoomControl,
  Circle,
  GeoJSON,
  GeoJSONProps,
} from 'react-leaflet'

import Locate from './Locate'

export default function Interface() {
  // const [instances, setInstances] = React.useState<Instance[]>([])
  const [geojson, setGeojson] = React.useState<GeoJSONType>({
    type: 'FeatureCollection',
    features: [],
  })
  const map = useMap()

  useEffect(() => {
    // getData<Instance[]>('/instances').then((res) => setInstances(res))
    // getData<GeoJSONType>('/solution.geojson').then((res) => setGeojson(res))
    getGeojson().then((res) => setGeojson(res))
  }, [])

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
            center={feature.geometry.coordinates.slice().reverse() as [number, number]}
            radius={80}
            color="blue"
            fillColor="blue"
            fillOpacity={0.5}
          />
        ) : null,
      )}
      <Locate map={map} />
      <ZoomControl position="bottomright" />
    </>
  )
}
