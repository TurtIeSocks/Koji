import * as React from 'react'
import {
  FormDataConsumer,
  FunctionField,
  ReferenceInput,
  SelectInput,
  TextInput,
} from 'react-admin'
import { Box, TextField } from '@mui/material'

import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import type { KojiRoute } from '@assets/types'
import { useStatic } from '@hooks/useStatic'

import CodeInput from '../inputs/CodeInput'
import RouteMap from './RouteMap'

export default function RouteForm() {
  const { scannerType } = useStatic.getState()

  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="description" fullWidth />
      <SelectInput
        source="mode"
        choices={(scannerType === 'rdm' ? RDM_ROUTES : UNOWN_ROUTES).map(
          (mode, i) => ({ id: i, mode }),
        )}
        optionText="mode"
        optionValue="mode"
      />
      <ReferenceInput source="geofence_id" reference="geofence" isRequired />
      <FunctionField
        label="Points"
        render={(fence: KojiRoute) => {
          const points: number =
            typeof fence?.geometry === 'string'
              ? JSON.parse(fence?.geometry || '{}')?.coordinates?.length || 0
              : fence?.geometry?.coordinates?.length || 0
          return (
            <TextField
              value={points}
              disabled
              fullWidth
              size="small"
              helperText="Point Count"
            />
          )
        }}
      />
      <CodeInput
        source="geometry"
        label="Route"
        conversionType="geometry"
        geometryType="MultiPoint"
      />
      <Box pt="1em" />
      <FormDataConsumer<KojiRoute>>
        {({ formData }) => <RouteMap formData={formData} />}
      </FormDataConsumer>
    </>
  )
}
