/* eslint-disable no-nested-ternary */
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
} from '@mui/material'
import CheckBoxOutlineBlank from '@mui/icons-material/CheckBoxOutlineBlank'
import IndeterminateCheckBoxOutlined from '@mui/icons-material/IndeterminateCheckBoxOutlined'
import CheckBox from '@mui/icons-material/CheckBox'
import type { Feature, GeoJsonTypes } from 'geojson'

import { KojiResponse } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { getData } from '@services/fetches'

const icon = <CheckBoxOutlineBlank fontSize="small" color="primary" />
const checkedIcon = <CheckBox fontSize="small" color="primary" />
const partialIcon = (
  <IndeterminateCheckBoxOutlined fontSize="small" color="primary" />
)
const filterOptions = createFilterOptions({
  matchFrom: 'any',
  stringify: (option: string) => option,
})

interface Option {
  id: number
  type: string
  name: string
  geoType?: Exclude<GeoJsonTypes, 'Feature' | 'FeatureCollection'>
}

interface Props {
  endpoint: string
  koji?: boolean
}

export default function InstanceSelect({ endpoint, koji = false }: Props) {
  const add = useShapes((s) => s.setters.add)
  const remove = useShapes((s) => s.setters.remove)

  const [loading, setLoading] = React.useState(false)
  const [options, setOptions] = React.useState<Record<string, Option>>({})
  const [selected, setSelected] = React.useState<string[]>([])

  React.useEffect(() => {
    setLoading(true)
    getData<KojiResponse<Option[]>>(endpoint)
      .then((resp) => {
        if (resp) {
          setOptions(
            Object.fromEntries(
              resp.data.map((o) => [`${o.name}__${o.type}`, o]),
            ),
          )
        }
        setLoading(false)
      })
      // eslint-disable-next-line no-console
      .catch((e) => console.error(e))
  }, [])

  const updateState = async (newValue: string[]) => {
    const added = newValue.filter((s) => !selected.includes(s))
    const deleted = selected.filter((s) => !newValue.includes(s))
    await Promise.all(
      added.map((a) =>
        getData<KojiResponse<Feature>>(
          `/internal/routes/one/${koji ? 'koji' : 'scanner'}/${options[
            a
          ].name.replace(/\//g, '__')}/${options[a].type}`,
        ).then((resp) => {
          if (resp) {
            add(resp.data, koji ? '' : '__SCANNER')
            setOptions((prev) => ({
              ...prev,
              [a]: { ...prev[a], geoType: resp.data.geometry.type },
            }))
          }
        }),
      ),
    )
    deleted.forEach((d) => {
      const { name, type, geoType } = options[d]
      if (geoType) {
        remove(geoType, `${name}${type}${koji ? '' : '__SCANNER'}`)
      }
    })
    setSelected(newValue)
  }

  return (
    <ListItem>
      <Autocomplete
        value={selected}
        size="small"
        onChange={async (_e, newValue) => updateState(newValue)}
        filterOptions={filterOptions}
        selectOnFocus
        clearOnBlur
        multiple
        loading={loading}
        handleHomeEndKeys
        disableCloseOnSelect
        fullWidth
        groupBy={(option) => options[option]?.type}
        options={Object.keys(options).sort((a, b) =>
          options[a].type?.localeCompare(options[b].type),
        )}
        renderTags={(val) => (
          <Typography align="center">({val.length})</Typography>
        )}
        renderOption={(props, option, { selected: s }) => {
          return (
            <li {...props}>
              <Checkbox
                icon={icon}
                checkedIcon={checkedIcon}
                style={{ marginRight: 8 }}
                checked={s}
              />
              {option.split('__')[0]}
            </li>
          )
        }}
        renderInput={(params) => (
          <TextField label="Select Instance" {...params} />
        )}
        renderGroup={({ key, group, children }) => {
          const allValues = Array.isArray(children)
            ? [...selected, ...children.map((x) => x.key)] // vaguely hacky way to select all filtered results
            : Object.values(options).filter((v) => v.type === group)
          const allSelected = allValues.every((v) => selected.includes(v))
          const partialSelected =
            allSelected || selected.some((v) => options[v]?.type === group)
          return group ? (
            <List key={key}>
              <ListItemButton
                onClick={() =>
                  updateState(
                    allSelected || partialSelected
                      ? selected.filter(
                          (v) =>
                            !allValues.includes(v) ||
                            options[v]?.type !== group,
                        )
                      : [
                          ...allValues,
                          ...selected.filter((v) => options[v]?.type !== group),
                        ],
                  )
                }
              >
                <ListItemIcon>
                  {allSelected
                    ? checkedIcon
                    : partialSelected
                    ? partialIcon
                    : icon}
                </ListItemIcon>
                <ListItemText
                  primary={capitalize(
                    group
                      .split('_')
                      .map((x) => capitalize(x))
                      .join(' '),
                  )}
                />
              </ListItemButton>
              {children}
            </List>
          ) : null
        }}
      />
    </ListItem>
  )
}
