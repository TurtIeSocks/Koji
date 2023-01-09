import * as React from 'react'
import {
  ArrayInput,
  FormDataConsumer,
  SelectInput,
  SimpleFormIterator,
  TextInput,
} from 'react-admin'
import { Box } from '@mui/material'
import Map from '@components/Map'
import { GeoJSON } from 'react-leaflet'
import center from '@turf/center'
import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import { safeParse } from '@services/utils'
import type { Feature } from 'geojson'

import CodeInput from '../inputs/CodeInput'

export default function GeofenceForm() {
  const scannerType = useStatic((s) => s.scannerType)
  return (
    <>
      <TextInput source="name" fullWidth required />
      <SelectInput
        source="mode"
        choices={(scannerType === 'rdm' ? RDM_FENCES : UNOWN_FENCES).map(
          (mode, i) => ({ id: i, mode }),
        )}
        optionText="mode"
        optionValue="mode"
      />
      <FormDataConsumer>
        {({ formData }) => {
          console.log(formData.area)
          const parsed =
            typeof formData.area === 'string'
              ? (() => {
                  const safe = safeParse<Feature>(formData.area)
                  if (!safe.error) return safe.value
                })()
              : formData.area
          if (parsed?.geometry === undefined) return null

          const point = center(parsed.geometry)
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
              <GeoJSON data={parsed} />
            </Map>
          )
        }}
      </FormDataConsumer>
      <Box pt="1em" />
      <ArrayInput source="properties">
        <SimpleFormIterator inline>
          <TextInput source="key" helperText={false} />
          <TextInput source="value" helperText={false} />
        </SimpleFormIterator>
      </ArrayInput>
      <CodeInput source="area" label="Fence" />
    </>
  )
}
