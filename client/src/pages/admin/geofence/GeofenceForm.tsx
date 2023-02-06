/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable react/jsx-no-useless-fragment */
import * as React from 'react'
import {
  ArrayInput,
  AutocompleteInput,
  BooleanInput,
  FormDataConsumer,
  NumberInput,
  ReferenceInput,
  SelectInput,
  SimpleFormIterator,
  TextInput,
  useRecordContext,
} from 'react-admin'
import { ColorInput } from 'react-admin-color-picker'
import { JsonInput } from 'react-admin-json-view'
import { Box, useTheme } from '@mui/material'
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

export default function GeofenceForm() {
  const scannerType = useStatic((s) => s.scannerType)
  const theme = useTheme()
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

  // console.log({ properties })
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
            source="id"
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
            {({ getSource, scopedFormData }) => {
              const id: number = scopedFormData?.id || 1
              // const name = scopedFormData?.name
              // const category: string = scopedFormData?.category || ''
              // console.log({ formData, scopedFormData })
              // console.log({ id, name, category }, properties[id]?.category)
              return (
                getSource && (
                  <>
                    {
                      {
                        boolean: <BooleanInput source={getSource('value')} />,
                        string: <TextInput source={getSource('value')} />,
                        number: <NumberInput source={getSource('value')} />,
                        object: (
                          <JsonInput
                            reactJsonOptions={{
                              theme:
                                theme.palette.mode === 'dark'
                                  ? 'chalk'
                                  : 'flat',
                            }}
                            source={getSource('value')}
                          />
                        ),
                        array: <JsonInput source={getSource('value')} />,
                        color: <ColorInput source={getSource('value')} />,
                        database: (
                          <TextInput disabled source={getSource('value')} />
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
