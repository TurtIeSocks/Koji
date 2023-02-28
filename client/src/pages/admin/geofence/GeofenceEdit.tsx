import * as React from 'react'
import {
  AutocompleteArrayInput,
  Edit,
  ReferenceArrayInput,
  SimpleForm,
  useRecordContext,
} from 'react-admin'

import type { AdminGeofence, KojiProject } from '@assets/types'

import GeofenceForm from './GeofenceForm'

function OptionRenderer() {
  const record = useRecordContext()
  return <span>{record.name}</span>
}
const inputText = (choice: KojiProject) => choice.name

const matchSuggestion = (filter: string, choice: KojiProject) => {
  return choice.name.toLowerCase().includes(filter.toLowerCase())
}

const transformPayload = async (geofence: AdminGeofence) => {
  return {
    ...geofence,
    geometry: {
      ...(typeof geofence.geometry === 'string'
        ? JSON.parse(geofence.geometry)
        : geofence.geometry),
    },
  }
}

export default function GeofenceEdit() {
  return (
    <Edit mutationMode="pessimistic" transform={transformPayload}>
      <SimpleForm>
        <GeofenceForm />
        <ReferenceArrayInput
          source="projects"
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
            sx={{ mt: 3 }}
          />
        </ReferenceArrayInput>
      </SimpleForm>
    </Edit>
  )
}
