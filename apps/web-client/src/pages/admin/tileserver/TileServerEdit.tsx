import * as React from 'react'
import { Edit, SimpleForm } from 'react-admin'

import TileServerForm from './TileServerForm'

export default function TileServerEdit() {
  return (
    <Edit mutationMode="pessimistic">
      <SimpleForm>
        <TileServerForm />
      </SimpleForm>
    </Edit>
  )
}
