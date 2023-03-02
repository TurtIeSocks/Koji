import { usePersist } from '@hooks/usePersist'
import * as React from 'react'
import { FormDataConsumer, TextInput } from 'react-admin'
import { MapContainer, TileLayer } from 'react-leaflet'

export default function TileServerForm() {
  const location = usePersist((s) => s.location)

  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="url" fullWidth isRequired />
      <FormDataConsumer>
        {({ formData }) => {
          return (
            <MapContainer center={location}>
              <TileLayer
                url={
                  formData.url ||
                  'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png'
                }
              />
            </MapContainer>
          )
        }}
      </FormDataConsumer>
    </>
  )
}
