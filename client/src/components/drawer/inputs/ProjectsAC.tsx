import { KojiProject, KojiResponse } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import {
  Autocomplete,
  Checkbox,
  CircularProgress,
  TextField,
  createFilterOptions,
} from '@mui/material'
import { fetchWrapper } from '@services/fetches'
import * as React from 'react'

interface NewKojiProject extends KojiProject {
  inputValue?: string
}

const filter = createFilterOptions<NewKojiProject>()

export default function ProjectsAc({
  value,
  setValue,
}: {
  value: number[]
  setValue: (value: number[]) => void
}) {
  const projectObj = useStatic((s) => s.projects)

  const [open, setOpen] = React.useState(false)
  const [projects, setProjects] = React.useState<KojiProject[]>(
    Object.values(projectObj),
  )
  const [loading, setLoading] = React.useState(false)

  const saveProject = async (newProject: NewKojiProject) => {
    const res = await fetchWrapper<KojiResponse<KojiProject>>(
      '/internal/admin/project/',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          id: 0,
          name: newProject.inputValue,
          created_at: new Date(),
          updated_at: new Date(),
        }),
      },
    )
    return res?.data
  }

  const getOptions = async (search = '') => {
    setLoading(true)
    const res = await fetchWrapper<KojiResponse<KojiProject[]>>(
      `/internal/admin/search/project/?query=${search}`,
    )
    if (res) {
      setProjects(res.data)
      setLoading(false)
    }
  }

  return (
    <Autocomplete
      value={value.map((id) => projectObj[id])}
      onChange={(_e, newValue) => {
        const newProject = newValue.find(
          (val) => typeof val === 'object' && val.id === 0,
        )
        if (typeof newProject === 'object') {
          saveProject(newProject).then((val) => {
            if (val) {
              const newProjects = Object.fromEntries(
                [...Object.values(projectObj), val].map((project) => [
                  project.id,
                  { ...project, geofences: [] },
                ]),
              )
              setValue(
                newValue.map((project) => {
                  return typeof project === 'string'
                    ? Object.values(newProjects).find(
                        (proj) => proj.name === project,
                      )?.id || val.id
                    : project.id || val.id
                }),
              )
              useStatic.setState({ projects: newProjects })
            }
          })
        } else {
          setValue(
            newValue.map((val) =>
              typeof val === 'string'
                ? Object.values(projectObj).find((proj) => proj.name === val)
                    ?.id || 0
                : val.id,
            ),
          )
        }
      }}
      onInputChange={(_e, newInputValue) => getOptions(newInputValue)}
      renderInput={(params) => (
        <TextField
          {...params}
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
          helperText={
            Object.values(projectObj).length === 0
              ? 'No projects found, try creating a new one'
              : undefined
          }
        />
      )}
      options={projects}
      getOptionLabel={(option) =>
        typeof option === 'string' ? option : option?.name
      }
      isOptionEqualToValue={(option, v) => option.name === v.name}
      filterOptions={(options, params) => {
        const filtered = filter(options, params)
        const { inputValue } = params
        const isExisting = projects.some((option) => inputValue === option.name)
        if (inputValue !== '' && !isExisting) {
          filtered.push({
            id: 0,
            name: `Add "${inputValue}"`,
            inputValue,
            created_at: new Date(),
            updated_at: new Date(),
            scanner: false,
          })
        }
        return filtered
      }}
      renderOption={(props, option, { selected }) => {
        return (
          <li {...props}>
            <Checkbox style={{ marginRight: 8 }} checked={selected} />
            {option.name}
          </li>
        )
      }}
      open={open}
      onFocus={() => {
        setOpen(true)
      }}
      onOpen={async () => {
        setOpen(true)
        await getOptions()
      }}
      onClose={() => {
        setOpen(false)
      }}
      limitTags={1}
      loading={loading}
      sx={{ width: '80%', mx: 'auto' }}
      size="small"
      multiple
      freeSolo
      disableCloseOnSelect
    />
  )
}
