import * as React from 'react'
import { FormDataConsumer, SelectInput, TextInput } from 'react-admin'
import { Box } from '@mui/material'
import type { MultiPoint } from 'geojson'
import center from '@turf/center'

import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import type { FeatureCollection } from '@assets/types'
import GeoJsonWrapper from '@components/GeojsonWrapper'
import Map from '@components/Map'
import { useStatic } from '@hooks/useStatic'
import { safeParse } from '@services/utils'

import CodeInput from '../inputs/CodeInput'

export default function RouteForm() {
  const scannerType = useStatic((s) => s.scannerType)

  return (
    <>
      <TextInput source="name" fullWidth required />
      <TextInput source="description" fullWidth />
      <SelectInput
        source="mode"
        choices={(scannerType === 'rdm' ? RDM_ROUTES : UNOWN_ROUTES).map(
          (mode, i) => ({ id: i, mode }),
        )}
        optionText="mode"
        optionValue="mode"
      />
      <FormDataConsumer>
        {({ formData }) => {
          const parsed: MultiPoint =
            typeof formData.geometry === 'string'
              ? (() => {
                  const safe = safeParse<MultiPoint>(formData.geometry)
                  if (!safe.error) return safe.value
                })()
              : formData.geometry
          if (parsed === undefined || !parsed.coordinates.length) return null
          const point = center(parsed)

          const fc: FeatureCollection = {
            type: 'FeatureCollection',
            features: (parsed.coordinates || []).map((c, i) => {
              return {
                id: `${i}`,
                type: 'Feature',
                geometry: {
                  type: 'Point',
                  coordinates: c,
                },
                properties: {
                  next: parsed.coordinates[
                    i === parsed.coordinates.length - 1 ? 0 : i + 1
                  ],
                },
              }
            }),
          }
          return (
            <Map
              forcedLocation={[
                point.geometry.coordinates[1],
                point.geometry.coordinates[0],
              ]}
              forcedZoom={8}
              zoomControl
              style={{ width: '100%', height: '50vh' }}
            >
              <GeoJsonWrapper fc={fc} mode={formData.mode} />
            </Map>
          )
        }}
      </FormDataConsumer>
      <Box pt="1em" />
      <CodeInput
        source="geometry"
        label="Route"
        conversionType="geometry"
        geometryType="MultiPoint"
      />
    </>
  )
}
