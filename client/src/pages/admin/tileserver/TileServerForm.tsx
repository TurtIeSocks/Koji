import * as React from 'react'
import { FormDataConsumer, TextInput } from 'react-admin'
import TileServerMap from './TileServerMap'

export default function TileServerForm() {
  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="url" fullWidth isRequired />
      <FormDataConsumer>
        {({ formData }) => <TileServerMap formData={formData} />}
      </FormDataConsumer>
    </>
  )
}
