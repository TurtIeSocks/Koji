import * as React from 'react'
import {
  ArrayInput,
  FormDataConsumer,
  ReferenceArrayInput,
  SelectArrayInput,
  SimpleForm,
  SimpleFormIterator,
  TextInput,
} from 'react-admin'
import { Box } from '@mui/material'
import Map from '@components/Map'
import { GeoJSON } from 'react-leaflet'
import center from '@turf/center'

export default function GeofenceForm() {
  return (
    <SimpleForm>
      <TextInput source="name" fullWidth required />
      <FormDataConsumer>
        {({ formData }) => {
          if (formData?.area?.geometry === undefined) return null
          const point = center(formData.area.geometry)
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
              <GeoJSON data={formData.area} />
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
      <ReferenceArrayInput
        source="related"
        reference="project"
        label="Projects"
      >
        <SelectArrayInput optionText="name" />
      </ReferenceArrayInput>
    </SimpleForm>
  )
}
