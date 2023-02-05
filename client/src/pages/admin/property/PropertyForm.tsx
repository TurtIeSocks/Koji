import * as React from 'react'
import { SelectInput, TextInput, useRecordContext } from 'react-admin'
import { Box } from '@mui/material'
import { PROPERTY_CATEGORIES } from '@assets/constants'
import { KojiProperty } from '@assets/types'

import CodeInput from '../inputs/CodeInput'

export default function PropertyForm() {
  const record = useRecordContext<KojiProperty>()

  const [tempState, setTempState] = React.useState(record.category)

  return (
    <>
      <TextInput source="name" fullWidth required />
      <SelectInput
        source="category"
        choices={PROPERTY_CATEGORIES.map((x, i) => ({ id: i, name: x }))}
        optionValue="name"
        optionText="name"
        required
        onChange={(e) => setTempState(e.target.value)}
      />
      {tempState !== 'Database' && (
        <CodeInput source="default_value" label="Default Value" />
      )}
      <Box pt="1em" />
    </>
  )
}
