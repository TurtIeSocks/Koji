import * as React from 'react'
import { Edit, SimpleForm } from 'react-admin'

import PropertyForm from './PropertyForm'

export default function PropertyEdit() {
  return (
    <Edit mutationMode="pessimistic">
      <SimpleForm>
        <PropertyForm />
      </SimpleForm>
    </Edit>
  )
}
