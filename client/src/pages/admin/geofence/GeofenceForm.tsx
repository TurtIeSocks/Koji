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
  useInput,
} from 'react-admin'

import { ColorInput } from 'react-admin-color-picker'
import { Box, Switch, TextField } from '@mui/material'
import center from '@turf/center'

import Map from '@components/Map'
import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import { safeParse } from '@services/utils'
import type { Feature, KojiProperty } from '@assets/types'
import GeoJsonWrapper from '@components/GeojsonWrapper'

import CodeInput from '../inputs/CodeInput'

function OptionRenderer() {
  const record = useRecordContext()
  return <span>{record.name}</span>
}
const inputText = (choice: KojiProperty) => choice.name

const matchSuggestion = (filter: string, choice: KojiProperty) => {
  return choice.name.toLowerCase().includes(filter.toLowerCase())
}

function BoolInputExpanded({
  source,
  defaultValue,
  name,
  ...props
}: {
  source: string
  name?: string
  defaultValue: boolean
}) {
  const { id, field } = useInput({ source })

  React.useEffect(() => {
    if (typeof field.value !== 'boolean') {
      field.onChange(defaultValue)
    }
  }, [name, defaultValue])

  return (
    <Switch
      id={id}
      {...props}
      {...field}
      onChange={({ target }) => {
        field.onChange({
          target: {
            value: target.checked,
          },
        })
      }}
      checked={!!(field.value ?? defaultValue)}
    />
  )
}

function TextInputExpanded({
  source,
  defaultValue,
  type = 'text',
  disabled = false,
  name,
  ...props
}: {
  source: string
  defaultValue: string | number
  type?: HTMLInputElement['type']
  disabled?: boolean
  name?: string
}) {
  const { id, field } = useInput({ source })

  React.useEffect(() => {
    if (
      !field.value ||
      typeof field.value !== (type === 'number' ? 'number' : 'string')
    ) {
      field.onChange(defaultValue)
    }
  }, [type, name, defaultValue])

  return (
    <TextField
      id={id}
      {...props}
      {...field}
      disabled={disabled}
      onChange={({ target }) => {
        field.onChange({
          target: {
            value: type === 'number' ? +target.value || 0 : target.value,
          },
        })
      }}
      type={type}
      value={(field.value ?? defaultValue) || (type === 'number' ? 0 : '')}
    />
  )
}

function ColorInputExpanded({
  source,
  defaultValue,
  name,
  ...props
}: {
  source: string
  defaultValue: string
  name?: string
}) {
  const { field } = useInput({ source, defaultValue })

  React.useEffect(() => {
    if (
      !field.value ||
      typeof field.value !== 'string' ||
      !field.value.startsWith('#') ||
      field.value.startsWith('rgb')
    ) {
      field.onChange(defaultValue)
    }
  }, [name, defaultValue])

  return <ColorInput {...props} source={source} />
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
      <FormDataConsumer>
        {({ formData }) => {
          const parsed =
            typeof formData.area === 'string'
              ? (() => {
                  const safe = safeParse<Feature>(formData.area)
                  if (!safe.error) return safe.value
                })()
              : formData.area
          if (parsed?.geometry === undefined) return null

          const point = center(parsed.geometry)
          return (
            <Map
              forcedLocation={[
                point.geometry.coordinates[1],
                point.geometry.coordinates[0],
              ]}
              forcedZoom={8}
              zoomControl
              style={{ width: '100%', height: '50vh' }}
            >
              <GeoJsonWrapper
                data={{ type: 'FeatureCollection', features: [parsed] }}
              />
            </Map>
          )
        }}
      </FormDataConsumer>
      <Box pt="1em" />
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
              // console.log(scopedFormData)
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
                            // editing={!!scopedFormData?.id}
                          />
                        ),
                        string: (
                          <TextInputExpanded
                            source={getSource('value')}
                            name={properties[id]?.name}
                            type="text"
                            defaultValue={properties[id]?.default_value || ''}
                            // editing={!!scopedFormData?.id}
                          />
                        ),
                        number: (
                          <TextInputExpanded
                            source={getSource('value')}
                            name={properties[id]?.name}
                            defaultValue={properties[id]?.default_value || 0}
                            type="number"
                            // editing={!!scopedFormData?.id}
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
                            defaultValue={properties[id]?.default_value || ''}
                            // editing={!!scopedFormData?.id}
                          />
                        ),
                        database: (
                          <TextInputExpanded
                            disabled
                            type="database"
                            source={getSource('value')}
                            name={properties[id]?.name}
                            defaultValue={
                              formData?.[properties[id]?.name] || ''
                            }
                            // editing={!!scopedFormData?.id}
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
    </>
  )
}
