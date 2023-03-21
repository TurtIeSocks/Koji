import * as React from 'react'
import { FormDataConsumer, SelectInput, TextInput } from 'react-admin'
import { Box } from '@mui/material'

import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import type { KojiRoute } from '@assets/types'
import { useStatic } from '@hooks/useStatic'

import CodeInput from '../inputs/CodeInput'
import RouteMap from './RouteMap'

export default function RouteForm() {
  const { scannerType } = useStatic.getState()

  return (
    <>
      <TextInput source="name" fullWidth required />
      <TextInput source="description" fullWidth />
      <SelectInput
        source="mode"
        choices={(scannerType === 'rdm' ? RDM_ROUTES : UNOWN_ROUTES).map(
          (mode, i) => ({ id: i, mode }),
        )}
        optionText="mode"
        optionValue="mode"
      />
      <FormDataConsumer<KojiRoute>>
        {({ formData }) => <RouteMap formData={formData} />}
      </FormDataConsumer>
      <Box pt="1em" />
      <CodeInput
        source="geometry"
        label="Route"
        conversionType="geometry"
        geometryType="MultiPoint"
      />
    </>
  )
}
