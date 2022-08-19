import * as React from 'react'
import {
  ListItem,
  Autocomplete,
  createFilterOptions,
  TextField,
  CircularProgress,
} from '@mui/material'

import type { UseStore } from '@hooks/useStore'
import { getData } from '@services/fetches'
import { useStatic } from '@hooks/useStatic'

interface Props {
  value: string
  setValue: (field: keyof UseStore, value: string) => void
}

export default function InstanceSelect({ value, setValue }: Props) {
  const instances = useStatic((s) => s.instances)
  const scannerType = useStatic((s) => s.scannerType)
  const setSettings = useStatic((s) => s.setSettings)

  const [loading, setLoading] = React.useState(false)
  React.useEffect(() => {
    if (!instances.length && scannerType === 'rdm') {
      setLoading(true)
      getData<string[]>('/api/instance/type/auto_quest').then((r) => {
        setSettings('instances', Array.isArray(r) ? r : [])
        setLoading(false)
      })
    }
  }, [])

  React.useEffect(() => {
    if (!instances.includes(value)) {
      setValue('instance', '')
    }
  }, [value])

  return (
    <ListItem>
      {loading ? (
        <CircularProgress color="secondary" size={100} />
      ) : (
        <Autocomplete
          value={instances.includes(value) ? value : ''}
          size="small"
          onChange={(_e, newValue) => {
            if (typeof newValue === 'string') setValue('instance', newValue)
          }}
          filterOptions={(opts, params) => {
            const filtered = createFilterOptions<string>()(opts, params)
            const { inputValue } = params
            const isExisting = opts.some((option) => inputValue === option)
            if (inputValue !== '' && !isExisting) {
              filtered.push(inputValue)
            }
            return filtered
          }}
          selectOnFocus
          clearOnBlur
          handleHomeEndKeys
          sx={{ width: '90%', mx: 'auto' }}
          options={['', ...instances]}
          renderOption={(props, option) => <li {...props}>{option}</li>}
          renderInput={(params) => (
            <TextField label="Select Instance" {...params} />
          )}
        />
      )}
    </ListItem>
  )
}
