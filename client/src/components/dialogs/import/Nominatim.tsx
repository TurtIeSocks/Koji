/* eslint-disable react/prop-types */
import { KojiResponse, Feature, FeatureCollection } from '@assets/types'
import {
  Autocomplete,
  Checkbox,
  CircularProgress,
  TextField,
  Typography,
} from '@mui/material'
import * as React from 'react'
import Clear from '@mui/icons-material/Clear'
import { getData } from '@services/fetches'

export default function Nominatim({
  features,
  handleChange,
}: {
  features: Feature[]
  handleChange: (geojson: FeatureCollection, key?: string) => void
}) {
  const [loading, setLoading] = React.useState(false)
  const [results, setResults] = React.useState<FeatureCollection>({
    type: 'FeatureCollection',
    features: [],
  })
  const [inputValue, setInputValue] = React.useState('')

  React.useEffect(() => {
    setLoading(true)
    getData<KojiResponse<FeatureCollection>>(
      `/config/nominatim?query=${inputValue}`,
    ).then((res) => {
      if (res) {
        setResults(res.data)
        setLoading(false)
      }
    })
  }, [inputValue])

  return (
    <Autocomplete
      value={features}
      onChange={(_e, newValue) => {
        const newGeojson: FeatureCollection = {
          type: 'FeatureCollection',
          features: [],
        }
        if (Array.isArray(newValue)) {
          newValue.forEach((feature) => {
            if (typeof feature === 'string') {
              return
            }
            newGeojson.features.push({
              ...feature,
              properties: {
                ...feature.properties,
                __nominatim: true,
              },
            })
          })
        }
        handleChange(newGeojson, '__nominatim')
      }}
      inputValue={inputValue}
      onInputChange={(_e, newInputValue) => {
        setInputValue(newInputValue)
      }}
      renderInput={(params) => (
        <TextField
          {...params}
          label="Search Nominatim for [Multi]Polygons"
          InputProps={{
            ...params.InputProps,
            endAdornment: (
              <>
                {loading ? (
                  <CircularProgress color="inherit" size={20} />
                ) : null}
                {params.InputProps.endAdornment}
              </>
            ),
          }}
        />
      )}
      options={results.features.sort((a, b) =>
        a.properties?.display_name.localeCompare(b.properties?.display_name),
      )}
      getOptionLabel={(option) => {
        return typeof option === 'string'
          ? option
          : option.properties?.display_name || ''
      }}
      renderOption={(props, option, { selected }) => {
        return (
          <li
            {...props}
            key={option.properties?.osm_id || option.properties?.display_name}
          >
            {props['aria-disabled'] ? (
              <Clear sx={{ mx: 1 }} />
            ) : (
              <Checkbox style={{ marginRight: 8 }} checked={selected} />
            )}
            <Typography variant="subtitle2">
              {option.properties?.display_name} - {option.geometry.type} (
              {option.properties?.osm_id})
            </Typography>
          </li>
        )
      }}
      getOptionDisabled={(option) =>
        option.geometry.type !== 'Polygon' &&
        option.geometry.type !== 'MultiPolygon'
      }
      isOptionEqualToValue={(option, v) =>
        option.properties?.osm_id === v.properties?.osm_id
      }
      limitTags={1}
      loading={loading}
      sx={{ width: '90%', mx: 'auto', my: 1 }}
      size="small"
      multiple
      freeSolo
      disableCloseOnSelect
      selectOnFocus
    />
  )
}
