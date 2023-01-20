import * as React from 'react'
import { Edit, SimpleForm } from 'react-admin'

import RouteForm from './RouteForm'

export default function RouteEdit() {
  return (
    <Edit mutationMode="pessimistic">
      <SimpleForm>
        <RouteForm />
      </SimpleForm>
    </Edit>
  )
}
