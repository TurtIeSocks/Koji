import * as React from 'react'
import {
  AutocompleteArrayInput,
  Edit,
  ReferenceArrayInput,
  SimpleForm,
  useRecordContext,
} from 'react-admin'

import { AdminGeofence, KojiProject } from '@assets/types'
import { fetchWrapper } from '@services/fetches'

import GeofenceForm from './GeofenceForm'

const transformPayload = async (geofence: AdminGeofence) => {
  if (Array.isArray(geofence.related)) {
    await fetchWrapper(
      `/internal/admin/geofence_project/geofence/${geofence.id}/`,
      {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(geofence.related),
      },
    )
  }
  return {
    ...geofence,
    area: {
      ...(typeof geofence.area === 'string'
        ? JSON.parse(geofence.area)
        : geofence.area),
      properties: Object.fromEntries(
        geofence.properties.map((p) => [p.key, p.value]),
      ),
    },
  }
}

function OptionRenderer() {
  const record = useRecordContext()
  return <span>{record.name}</span>
}
const inputText = (choice: KojiProject) => choice.name

const matchSuggestion = (filter: string, choice: KojiProject) => {
  return choice.name.toLowerCase().includes(filter.toLowerCase())
}

export default function GeofenceEdit() {
  return (
    <Edit mutationMode="pessimistic" transform={transformPayload}>
      <SimpleForm>
        <GeofenceForm />
        <ReferenceArrayInput
          source="related"
          reference="project"
          label="Projects"
          perPage={1000}
          sort={{ field: 'name', order: 'ASC' }}
          alwaysOn={false}
        >
          <AutocompleteArrayInput
            optionText={<OptionRenderer />}
            inputText={inputText}
            matchSuggestion={matchSuggestion}
            disableCloseOnSelect
            label="Related Projects"
            fullWidth
            blurOnSelect={false}
          />
        </ReferenceArrayInput>
      </SimpleForm>
    </Edit>
  )
}
