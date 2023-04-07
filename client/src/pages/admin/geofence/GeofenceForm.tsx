/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable react/jsx-no-useless-fragment */
import * as React from 'react'
import {
  ArrayInput,
  AutocompleteInput,
  FormDataConsumer,
  ReferenceInput,
  SelectInput,
  SimpleFormIterator,
  TextInput,
  useRecordContext,
} from 'react-admin'

import { Box } from '@mui/material'

import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import type { AdminGeofence, KojiProperty } from '@assets/types'

import CodeInput from '../inputs/CodeInput'
import {
  BoolInputExpanded,
  ColorInputExpanded,
  TextInputExpanded,
} from '../inputs/Properties'
import { GeofenceMap } from './GeofenceMap'

function OptionRenderer() {
  const record = useRecordContext()
  return <span>{record.name}</span>
}
const inputText = (choice: KojiProperty) => choice.name

const matchSuggestion = (filter: string, choice: KojiProperty) => {
  return choice.name.toLowerCase().includes(filter.toLowerCase())
}

export default function GeofenceForm() {
  const scannerType = useStatic((s) => s.scannerType)
  const [properties, setProperties] = React.useState<
    Record<string, KojiProperty>
  >({})

  React.useEffect(() => {
    fetch('/internal/admin/property/all/')
      .then((res) => res.json())
      .then((data) => {
        const newProperties: Record<string, KojiProperty> = {}
        data.data.forEach((property: KojiProperty) => {
          newProperties[property.id] = property
        })
        setProperties(newProperties)
      })
  }, [])

  return (
    <>
      <TextInput source="name" fullWidth required />
      <SelectInput
        source="mode"
        choices={(scannerType === 'rdm' ? RDM_FENCES : UNOWN_FENCES).map(
          (mode, i) => ({ id: i, mode }),
        )}
        optionText="mode"
        optionValue="mode"
      />
      <ReferenceInput source="parent" reference="geofence" />
      <ArrayInput source="properties" sx={{ my: 2 }}>
        <SimpleFormIterator inline>
          <ReferenceInput
            source="property_id"
            reference="property"
            label="Name"
            perPage={1000}
            sort={{ field: 'category', order: 'ASC' }}
          >
            <AutocompleteInput
              optionText={<OptionRenderer />}
              inputText={inputText}
              matchSuggestion={matchSuggestion}
              groupBy={(x: KojiProperty) => x.category}
              label="Name"
            />
          </ReferenceInput>
          <FormDataConsumer>
            {({ formData, getSource, scopedFormData }) => {
              const id: number = scopedFormData?.property_id || 1
              const defaultValue = properties[id]?.default_value
              return (
                getSource && (
                  <>
                    {
                      {
                        boolean: (
                          <BoolInputExpanded
                            source={getSource('value')}
                            name={properties[id]?.name}
                            defaultValue={!!properties[id]?.default_value}
                            label="Value"
                          />
                        ),
                        string: (
                          <TextInputExpanded
                            source={getSource('value')}
                            name={properties[id]?.name}
                            type="text"
                            label="Value"
                            defaultValue={
                              typeof defaultValue === 'string'
                                ? defaultValue
                                : ''
                            }
                          />
                        ),
                        number: (
                          <TextInputExpanded
                            source={getSource('value')}
                            name={properties[id]?.name}
                            label="Value"
                            defaultValue={
                              typeof defaultValue === 'number'
                                ? defaultValue
                                : ''
                            }
                            type="number"
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
                            source={getSource('value')}
                            name={properties[id]?.name}
                            label="Value"
                            defaultValue={
                              typeof defaultValue === 'string'
                                ? defaultValue
                                : '#000000'
                            }
                          />
                        ),
                        database: (
                          <TextInputExpanded
                            disabled
                            type="database"
                            source={getSource('value')}
                            name={properties[id]?.name}
                            label="Value"
                            defaultValue={
                              formData?.[properties[id]?.name] || ''
                            }
                          />
                        ),
                      }[properties[id]?.category?.toLowerCase()]
                    }
                  </>
                )
              )
            }}
          </FormDataConsumer>
        </SimpleFormIterator>
      </ArrayInput>
      <CodeInput
        source="geometry"
        label="Fence"
        conversionType="geometry"
        geometryType="Polygon"
      />
      <Box pt="1em" />
      <FormDataConsumer<AdminGeofence>>
        {({ formData }) => <GeofenceMap formData={formData} />}
      </FormDataConsumer>
    </>
  )
}
