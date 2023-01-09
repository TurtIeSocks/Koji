import * as React from 'react'
import {
  Edit,
  ReferenceArrayInput,
  SelectArrayInput,
  SimpleForm,
  TextInput,
} from 'react-admin'

import { ClientProject } from '@assets/types'

const transformPayload = async (project: ClientProject) => {
  if (Array.isArray(project.related)) {
    await fetch(`/internal/admin/geofence_project/project/${project.id}`, {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(project.related),
    })
  }
  return project
}

export default function ProjectEdit() {
  return (
    <Edit mutationMode="pessimistic" transform={transformPayload}>
      <SimpleForm>
        <TextInput source="name" fullWidth isRequired />
        <ReferenceArrayInput
          source="related"
          reference="geofence"
          label="Geofences"
        >
          <SelectArrayInput optionText="name" />
        </ReferenceArrayInput>
      </SimpleForm>
    </Edit>
  )
}
