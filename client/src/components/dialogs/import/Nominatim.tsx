import { KojiResponse } from '@assets/types'
import {
  Autocomplete,
  Checkbox,
  CircularProgress,
  TextField,
  Typography,
} from '@mui/material'
import * as React from 'react'
import type { Feature, FeatureCollection } from 'geojson'

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
    fetch(`/config/nominatim?query=${inputValue}`)
      .then((res) => res.json())
      .then((res: KojiResponse<FeatureCollection>) => {
        setResults(res.data)
        setLoading(false)
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
          label="Search Nominatim"
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
      options={results.features}
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
            <Checkbox style={{ marginRight: 8 }} checked={selected} />
            <Typography variant="subtitle2">
              {option.properties?.display_name} ({option.properties?.osm_id})
            </Typography>
          </li>
        )
      }}
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
