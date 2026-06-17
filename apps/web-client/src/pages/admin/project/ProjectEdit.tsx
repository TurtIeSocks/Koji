import * as React from 'react'
import {
  AutocompleteArrayInput,
  Edit,
  ReferenceArrayInput,
  SimpleForm,
  useRecordContext,
} from 'react-admin'

import { KojiGeofence } from '@assets/types'
import ProjectForm from './ProjectForm'

function OptionRenderer() {
  const record = useRecordContext()
  return <span>{record.name}</span>
}
const inputText = (choice: KojiGeofence) => choice.name

const matchSuggestion = (filter: string, choice: KojiGeofence) => {
  return choice.name.toLowerCase().includes(filter.toLowerCase())
}

export default function ProjectEdit() {
  return (
    <Edit mutationMode="pessimistic">
      <SimpleForm>
        <ProjectForm />
        <ReferenceArrayInput
          source="geofences"
          reference="geofence"
          label="Geofences"
          perPage={1000}
          sort={{ field: 'name', order: 'ASC' }}
          alwaysOn={false}
        >
          <AutocompleteArrayInput
            optionText={<OptionRenderer />}
            inputText={inputText}
            matchSuggestion={matchSuggestion}
            disableCloseOnSelect
            label="Related Geofences"
            fullWidth
            blurOnSelect={false}
          />
        </ReferenceArrayInput>
      </SimpleForm>
    </Edit>
  )
}
