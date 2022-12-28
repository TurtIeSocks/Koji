import * as React from 'react'
import {
  Autocomplete,
  createFilterOptions,
  TextField,
  Checkbox,
  Typography,
} from '@mui/material'
import CheckBoxOutlineBlank from '@mui/icons-material/CheckBoxOutlineBlank'
import CheckBox from '@mui/icons-material/CheckBox'

const icon = <CheckBoxOutlineBlank fontSize="small" color="primary" />
const checkedIcon = <CheckBox fontSize="small" color="primary" />
const filterOptions = createFilterOptions({
  matchFrom: 'any',
  stringify: (option: string) => option,
})

interface Props {
  selected: string[]
  options: Record<string, number>
  onChange: (
    e: React.SyntheticEvent<Element, Event>,
    newValue: string[],
  ) => void
  loading?: boolean
  label: string
}

export default function KojiAuto({
  selected,
  options,
  onChange,
  label,
  loading,
}: Props) {
  return (
    <Autocomplete
      value={selected}
      size="small"
      onChange={onChange}
      filterOptions={filterOptions}
      selectOnFocus
      clearOnBlur
      multiple
      loading={loading}
      handleHomeEndKeys
      disableCloseOnSelect
      fullWidth
      options={Object.keys(options)}
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
            {option}
          </li>
        )
      }}
      renderInput={(params) => <TextField label={label} {...params} />}
    />
  )
}
