import * as React from 'react'
import {
  ListItem,
  Autocomplete,
  createFilterOptions,
  TextField,
  Checkbox,
  Typography,
  CircularProgress,
} from '@mui/material'

import type {
  KojiResponse,
  KojiKey,
  Feature,
  GeometryTypes,
} from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { fetchWrapper } from '@services/fetches'
import { useDbCache } from '@hooks/useDbCache'

const { add, remove } = useShapes.getState().setters

export default function SelectProject({
  label = 'Select Project',
}: {
  label?: string
}) {
  const { feature: featureCache, geofence: geofenceCache } =
    useDbCache.getState()
  const { Polygon, MultiPolygon } = useShapes.getState()

  const options = useDbCache((s) => s.project)

  const [loading, setLoading] = React.useState(false)
  const [selected, setSelected] = React.useState<string[]>([])

  const filterOptions = createFilterOptions({
    matchFrom: 'any',
    stringify: (option: string) =>
      `${option}${options[option as KojiKey]?.name}`,
  })

  const updateState = async (newValue: string[]) => {
    const added = newValue.filter((s) => !selected.includes(s))
    const deleted = selected.filter((s) => !newValue.includes(s))

    const addFeatures = [
      ...new Set(
        added.flatMap((a) =>
          options[a].geofences?.map((r) => [
            geofenceCache[r].id,
            `${geofenceCache[r].id}__${geofenceCache[r].mode}__KOJI`,
          ]),
        ),
      ),
    ].filter((a): a is [number, KojiKey] => Boolean(a))
    const removeFeature = [
      ...new Set(
        deleted.flatMap((a) =>
          options[a].geofences?.map((r) => [
            geofenceCache[r].geo_type,
            `${geofenceCache[r].id}__${geofenceCache[r].mode}__KOJI`,
          ]),
        ),
      ),
    ].filter((a): a is [GeometryTypes | undefined, KojiKey] => Boolean(a))

    setLoading(true)
    const newFeatures = await Promise.allSettled(
      addFeatures.map(
        (a) =>
          featureCache[a[1]] ||
          fetchWrapper<KojiResponse<Feature>>(
            `/internal/routes/one/koji/${a[0]}/${
              geofenceCache[a[0]]?.mode || 'Unset'
            }&internal=true`,
          ).then((resp) => {
            return resp?.data
          }),
      ),
    ).then((res) => {
      setLoading(false)
      return res
    })

    const cleaned = newFeatures
      .filter(
        (result): result is PromiseFulfilledResult<Feature> =>
          result.status === 'fulfilled' && !!result.value,
      )
      .map((result) => result.value)

    setSelected(newValue)

    add(cleaned)
    removeFeature.forEach(([geo_type, id]) => {
      if (geo_type) remove(geo_type, id)
    })
  }

  return (
    <ListItem key={Object.keys(options).join('')} sx={{ py: 1 }}>
      <Autocomplete
        size="small"
        onChange={(_e, newValue) => newValue && updateState(newValue)}
        value={selected}
        filterOptions={filterOptions}
        selectOnFocus
        clearOnBlur
        multiple
        loading={loading}
        handleHomeEndKeys
        disableCloseOnSelect
        fullWidth
        options={Object.keys(options).sort((a, b) =>
          options[a].name.localeCompare(options[b].name),
        )}
        renderTags={(val) => (
          <Typography align="center">({val.length})</Typography>
        )}
        renderOption={(props, option, { selected: s }) => {
          const fullOption = options[option]
          const keys =
            fullOption.geofences?.map((id) => {
              const dbFence = geofenceCache[id]
              return `${dbFence.id}__${dbFence.mode}__KOJI` as KojiKey
            }) || []

          const allSelected =
            !!fullOption.geofences?.length &&
            keys.every((id) => Polygon[id] || MultiPolygon[id])
          const partialSelected =
            allSelected ||
            fullOption.geofences?.some((id) => {
              const dbFence = geofenceCache[id]
              const shapeKey: KojiKey = `${dbFence.id}__${dbFence.mode}__KOJI`
              return dbFence.geo_type === 'Polygon'
                ? Polygon[shapeKey]
                : MultiPolygon[shapeKey]
            })

          return (
            <li {...props} style={{ display: 'flex' }}>
              <div style={{ flexGrow: 1 }}>
                <Checkbox
                  indeterminate={!allSelected && partialSelected && s}
                  style={{ marginRight: 8 }}
                  checked={s}
                  disabled={loading}
                />

                {fullOption.name}
              </div>
              {loading && <CircularProgress size={20} sx={{ flexGrow: 0 }} />}
            </li>
          )
        }}
        renderInput={(params) => <TextField label={label} {...params} />}
      />
    </ListItem>
  )
}
