import * as React from 'react'
import { FormDataConsumer, TextInput } from 'react-admin'

import { KojiTileServer } from '@assets/types'

import TileServerMap from './TileServerMap'

export default function TileServerForm() {
  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="url" fullWidth isRequired />
      <FormDataConsumer<KojiTileServer>>
        {({ formData }) => <TileServerMap formData={formData} />}
      </FormDataConsumer>
    </>
  )
}
