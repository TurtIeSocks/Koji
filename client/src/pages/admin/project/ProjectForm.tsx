import * as React from 'react'
import {
  // ArrayInput,
  AutocompleteInput,
  // FormDataConsumer,
  SimpleForm,
  // SimpleFormIterator,
  // TextInput,
} from 'react-admin'

export default function ProjectForm() {
  const officiallySupported = [
    { id: 'ReactMap', name: 'ReactMap' },
    { id: 'Poracle', name: 'Poracle' },
  ]

  return (
    <SimpleForm>
      <AutocompleteInput
        source="name"
        fullWidth
        isRequired
        choices={officiallySupported}
        onCreate={(filter) => {
          if (filter) {
            const newCategory = {
              id: filter || '',
              name: filter || '',
            }
            officiallySupported.push(newCategory)
            return newCategory
          }
        }}
      />

      {/* <ArrayInput source="properties">
        <SimpleFormIterator inline>
          <TextInput source="key" helperText={false} />
          <TextInput source="value" helperText={false} />
        </SimpleFormIterator>
      </ArrayInput> */}
    </SimpleForm>
  )
}
