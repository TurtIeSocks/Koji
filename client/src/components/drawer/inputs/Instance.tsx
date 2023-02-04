import * as React from 'react'
import {
  ListItem,
  Autocomplete,
  createFilterOptions,
  TextField,
  Checkbox,
  Typography,
  List,
  ListItemText,
  ListItemButton,
  capitalize,
  ListItemIcon,
  CircularProgress,
} from '@mui/material'
import CheckBoxOutlineBlank from '@mui/icons-material/CheckBoxOutlineBlank'
import IndeterminateCheckBoxOutlined from '@mui/icons-material/IndeterminateCheckBoxOutlined'
import CheckBox from '@mui/icons-material/CheckBox'

import type {
  KojiResponse,
  KojiKey,
  Feature,
  FeatureCollection,
} from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { fetchWrapper } from '@services/fetches'
import { useDbCache } from '@hooks/useDbCache'

const icon = <CheckBoxOutlineBlank fontSize="small" color="primary" />
const checkedIcon = <CheckBox fontSize="small" color="primary" />
const partialIcon = (
  <IndeterminateCheckBoxOutlined fontSize="small" color="primary" />
)
export default function InstanceSelect({
  setGeojson,
  koji = false,
  controlled = false,
  // filters = [],
  initialState = [],
  label = 'Select Instance',
}: {
  setGeojson?: (collection: FeatureCollection, deleted: string[]) => void
  koji?: boolean
  filters?: readonly string[]
  controlled?: boolean
  initialState?: KojiKey[]
  label?: string
}) {
  const { add, remove } = useShapes.getState().setters
  const {
    feature: featureCache,
    setRecords,
    getFromKojiKey,
  } = useDbCache.getState()
  const options = useDbCache((s) =>
    koji ? s.getOptions('geofence', 'route') : s.getOptions('scanner'),
  )
  const [loading, setLoading] = React.useState(false)
  const [selected, setSelected] = React.useState<KojiKey[]>([])

  React.useEffect(() => {
    if (controlled) setSelected(initialState)
  }, [initialState])

  const filterOptions = createFilterOptions({
    matchFrom: 'any',
    stringify: (option: string) =>
      `${option}${options[option as KojiKey]?.name}`,
  })

  const updateState = async (newValue: KojiKey[]) => {
    const added = newValue.filter((s) => !selected.includes(s))
    const deleted = selected.filter((s) => !newValue.includes(s))

    setLoading(true)
    const newFeatures = await Promise.allSettled(
      added.map(
        (a) =>
          featureCache[a] ||
          fetchWrapper<KojiResponse<Feature>>(
            `/internal/routes/one/${koji ? 'koji' : 'scanner'}/${
              options[a].id
            }/${options[a].mode || 'Unset'}`,
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

    if (setGeojson) {
      setGeojson(
        {
          type: 'FeatureCollection',
          features: newValue
            .map((n) => featureCache[n] || cleaned.find((f) => f.id === n))
            .filter(Boolean),
        },
        deleted,
      )
    } else {
      add(cleaned, koji ? '__KOJI' : '__SCANNER')
      deleted.forEach((d) => {
        const { geo_type } = options[d]
        if (geo_type) {
          remove(geo_type, d)
        }
      })
    }
    if (controlled) setSelected(newValue)
    if (koji) {
      const { geofence, route } = useDbCache.getState()
      const updatedFences = { ...geofence }
      const updatedRoutes = { ...route }

      cleaned.forEach((c) => {
        const reference = getFromKojiKey(c.id.toString())
        if (reference) {
          if (c.geometry.type.includes('Polygon')) {
            updatedFences[c.id] = {
              ...reference,
              geo_type: c.geometry.type as 'Polygon' | 'MultiPolygon',
            }
          } else if (c.geometry.type === 'MultiPoint') {
            updatedRoutes[c.id] = {
              ...reference,
              geo_type: c.geometry.type as 'MultiPoint',
            }
          }
        }
      })
      setRecords('route', updatedRoutes)
      setRecords('geofence', updatedFences)
    } else {
      const { scanner } = useDbCache.getState()
      setRecords('scanner', {
        ...scanner,
        ...Object.fromEntries(
          cleaned.map((c) => [
            c.id,
            {
              ...getFromKojiKey(c.id.toString()),
              geo_type: c.geometry.type,
            },
          ]),
        ),
      })
    }
  }

  return (
    <ListItem
      key={initialState.some((s) => !options[s]).toString()}
      sx={{ py: 1 }}
    >
      <Autocomplete
        value={selected}
        size="small"
        onChange={(_e, newValue) => updateState(newValue as KojiKey[])}
        filterOptions={filterOptions}
        selectOnFocus
        clearOnBlur
        multiple
        loading={loading}
        handleHomeEndKeys
        disableCloseOnSelect
        fullWidth
        groupBy={(option) => options[option as KojiKey]?.mode || 'Unset'}
        options={Object.keys(options).sort((a, b) =>
          (options[a as KojiKey].mode || 'Unset').localeCompare(
            options[b as KojiKey].mode || 'Unset',
          ),
        )}
        renderTags={(val) => (
          <Typography align="center">({val.length})</Typography>
        )}
        renderOption={(props, option, { selected: s }) => {
          const fullOption = options[option as KojiKey]
          return (
            <li {...props} style={{ display: 'flex' }}>
              <div style={{ flexGrow: 1 }}>
                {controlled && (
                  <Checkbox
                    icon={icon}
                    checkedIcon={checkedIcon}
                    style={{ marginRight: 8 }}
                    checked={s}
                    disabled={loading}
                  />
                )}
                {fullOption.name}
                {
                  {
                    Polygon: '(P)',
                    MultiPolygon: '(MP)',
                    Point: '',
                    MultiPoint: '',
                    GeometryCollection: '',
                    LineString: '',
                    MultiLineString: '',
                  }[options[option as KojiKey]?.geo_type || 'Point']
                }
              </div>
              {loading && <CircularProgress size={20} sx={{ flexGrow: 0 }} />}
            </li>
          )
        }}
        renderInput={(params) => <TextField label={label} {...params} />}
        renderGroup={({ key, group, children }) => {
          const allValues = Array.isArray(children)
            ? [...selected, ...children.map((x) => x.key)] // vaguely hacky way to select all filtered results
            : Object.values(options).filter((v) => v.mode === group)
          const allSelected = allValues.every((v) => selected.includes(v))
          const partialSelected =
            allSelected || selected.some((v) => options[v]?.mode === group)
          return group ? (
            <List key={key}>
              <ListItemButton
                disabled={loading}
                onClick={() =>
                  updateState(
                    allSelected || partialSelected
                      ? selected.filter(
                          (v) =>
                            !allValues.includes(v) ||
                            (options[v]?.mode || 'Unset') !== group,
                        )
                      : [
                          ...allValues,
                          ...selected.filter(
                            (v) => (options[v]?.mode || 'Unset') !== group,
                          ),
                        ],
                  )
                }
              >
                {controlled && (
                  <ListItemIcon>
                    {allSelected
                      ? checkedIcon
                      : partialSelected
                      ? partialIcon
                      : icon}
                  </ListItemIcon>
                )}
                <ListItemText
                  primary={capitalize(
                    group
                      .split('_')
                      .map((x) => capitalize(x))
                      .join(' '),
                  )}
                />
                {loading && <CircularProgress size={20} />}
              </ListItemButton>
              {Array.isArray(children)
                ? children.sort((a, b) =>
                    options[a.key].name.localeCompare(options[b.key].name),
                  )
                : children}
            </List>
          ) : null
        }}
      />
    </ListItem>
  )
}
