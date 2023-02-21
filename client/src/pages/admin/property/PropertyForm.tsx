import * as React from 'react'
import { SelectInput, TextInput, useRecordContext } from 'react-admin'
import { Box, Typography } from '@mui/material'
import { PROPERTY_CATEGORIES } from '@assets/constants'
import { KojiProperty } from '@assets/types'
import {
  BoolInputExpanded,
  ColorInputExpanded,
  TextInputExpanded,
} from '../inputs/Properties'

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
      {
        {
          boolean: (
            <BoolInputExpanded
              source="default_value"
              defaultValue={false}
              label="Default Value"
            />
          ),
          string: (
            <TextInputExpanded
              source="default_value"
              type="text"
              defaultValue=""
              label="Default Value"
            />
          ),
          number: (
            <TextInputExpanded
              source="default_value"
              defaultValue={0}
              type="number"
              label="Default Value"
            />
          ),
          object: (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <div>Not Implemented</div>
            </div>
          ),
          array: <div>Not Implemented</div>,
          color: (
            <ColorInputExpanded
              source="default_value"
              defaultValue="#000000"
              label="Default Value"
            />
          ),
          database: (
            <Typography>
              This value is set automatically based off of the name.
              <br />
              Example: `name` = mode, this property will automatically grab the
              value from the `mode` column of the Geofence table.
            </Typography>
          ),
        }[tempState.toLowerCase()]
      }
      {record.category !== tempState && (
        <Typography color="error">
          Changing the category will reset all values associated with this
          property to the new `default_value`
        </Typography>
      )}
      <Box pt="1em" />
    </>
  )
}
