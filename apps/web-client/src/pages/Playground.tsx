/* eslint-disable no-console */
import * as React from 'react'
import Grid from '@mui/material/Unstable_Grid2/Grid2'
import { Code } from '@components/Code'
import {
  Button,
  Collapse,
  Divider,
  IconButton,
  List,
  ListItem,
  ListItemText,
  MenuItem,
  Select,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import { stringify } from 'querystring'

import { ConversionOptions, Feature, KojiProject, Poracle } from '@assets/types'
import { CONVERSION_TYPES } from '@assets/constants'
import Subheader from '@components/styled/Subheader'
import { useNavigate } from 'react-router'
import ArrowBack from '@mui/icons-material/ArrowBack'
import ContentCopyIcon from '@mui/icons-material/ContentCopy'

interface Props {
  item: string
  params: { [key: string]: string | boolean | number }
  setParams: React.Dispatch<React.SetStateAction<Props['params']>>
}

function BaseItem({
  item,
  children,
}: {
  item: Props['item']
  children: React.ReactNode
}) {
  return (
    <ListItem key={item} dense>
      <ListItemText primary={item} />
      {children}
    </ListItem>
  )
}

function BoolItem({ item, params, setParams }: Props) {
  return (
    <BaseItem item={item}>
      <Switch
        edge="end"
        onChange={(_e, v) => setParams((prev) => ({ ...prev, [item]: v }))}
        checked={!!params[item]}
      />
    </BaseItem>
  )
}

function TextItem({
  item,
  type,
  params,
  setParams,
}: Props & { type: 'text' | 'number' }) {
  return (
    <BaseItem item={item}>
      <TextField
        size="small"
        type={type}
        color="secondary"
        error={!!params[item]}
        value={params[item] || ''}
        onChange={({ target }) =>
          setParams((prev) => ({
            ...prev,
            [item]: type === 'number' ? +target.value : target.value,
          }))
        }
      />
    </BaseItem>
  )
}

const PROPERTIES = {
  id: 'boolean',
  name: 'boolean',
  mode: 'boolean',
  parent: 'boolean',
  group: 'boolean',
  fullcoords: 'boolean',
}

const PARAMS = {
  trimstart: 'number',
  trimend: 'number',
  alphanumeric: 'boolean',
  replace: 'string',
  parentreplace: 'string',
  parentstart: 'string',
  parentend: 'string',
  lowercase: 'boolean',
  uppercase: 'boolean',
  capfirst: 'boolean',
  capitalize: 'string',
  underscore: 'string',
  dash: 'string',
  space: 'string',
  exclude: 'string',
  excludeparents: 'string',
  excludeproperties: 'string',
  unpolish: 'boolean',
  ignoremanualparent: 'boolean',
}

export default function Playground() {
  const navigate = useNavigate()
  const [code, setCode] = React.useState('')
  const [project, setProject] = React.useState('')
  const [projects, setProjects] = React.useState<KojiProject[]>([])
  const [type, setType] = React.useState<ConversionOptions>('featureCollection')
  const [params, setParams] = React.useState<Props['params']>({})
  const [namesOnly, setNamesOnly] = React.useState(false)
  const [error, setError] = React.useState('')

  // TODO(v2-gap): this dev API playground exercised the v1 geofence
  // name-modifier export (`/api/v1/geofence/{type}/{project}` + the trim/replace/
  // parent* PARAMS), which was REMOVED in v2 (the name_modifier wrapper is gone).
  // There is no v2 endpoint that takes a project name + name-modifier params, so
  // the modifier params below are dropped and the request best-effort hits the
  // geofences list as `?format={type}` (project-scoped export is unavailable).
  // The whole Playground is a dev tool; flagged for the user to decide whether to
  // retire it or build a v2 equivalent.
  const url = `/api/v2/geofences?format=${type}&${stringify(
    Object.fromEntries(Object.entries(params).filter(([, v]) => v)),
  )}`

  React.useEffect(() => {
    // v2 `GET /api/v2/projects` → row records inside the envelope.
    fetch('/api/v2/projects?per_page=9999')
      .then(async (res) => {
        if (!res.ok) {
          const err = await res.text()
          return setError(err)
        }
        return res.json()
      })
      .then((res) => {
        const data = res?.status === 'ok' ? res.data : res.data
        if (data?.[0]) setProject(data[0].name)
        setProjects(data || [])
        setError('')
      })
  }, [])

  React.useEffect(() => {
    fetch(url)
      .then(async (res) => {
        if (!res.ok) {
          const err = await res.text()
          return setError(err)
        }
        return res.json()
      })
      .then((res) => {
        if (namesOnly) {
          if (type === 'featureCollection') {
            res.data = res.data.features.map(
              (feat: Feature) => feat.properties.name,
            )
          } else if (type === 'feature_vec') {
            res.data = res.data.map((feat: Feature) => feat.properties.name)
          } else if (type === 'poracle') {
            res.data = res.data.map((feat: Poracle) => feat.name)
          }
        }
        setError('')
        return setCode(JSON.stringify(res, null, 2))
      })
  }, [url, namesOnly])

  return (
    <Grid container width="100%">
      <Grid
        container
        direction="column"
        xs={12}
        sm={6}
        // md={5}
        sx={{
          height: '100vh',
          '& > *': {
            width: '90%',
            margin: '0.5rem',
          },
        }}
      >
        <Grid container direction="row" xs={12} py={2}>
          <Grid xs={6} sm={4} md={3}>
            <Button onClick={() => navigate('/')} startIcon={<ArrowBack />}>
              Back
            </Button>
          </Grid>
          <Grid xs={6} sm={4} md={5}>
            <Typography>Kōji API Playground</Typography>
          </Grid>
          <Grid xs={12} sm={4}>
            <Switch
              checked={namesOnly}
              size="small"
              onChange={(_e, value) => {
                if (
                  value &&
                  !['poracle', 'feature_vec', 'featureCollection'].includes(
                    type,
                  )
                ) {
                  setType('featureCollection')
                }
                return setNamesOnly(value)
              }}
            />
            <Typography variant="caption">Names Only</Typography>
          </Grid>
        </Grid>
        <Grid>
          <Select
            value={project}
            fullWidth
            size="small"
            onChange={({ target }) => setProject(target.value)}
          >
            {projects.map((proj) => (
              <MenuItem key={proj.name} value={proj.name}>
                {proj.name}
              </MenuItem>
            ))}
          </Select>
        </Grid>
        <Grid>
          <Collapse in={!namesOnly} dir="down">
            <Select
              value={type}
              fullWidth
              size="small"
              onChange={({ target }) =>
                setType(target.value as ConversionOptions)
              }
            >
              {CONVERSION_TYPES.map((opt) => (
                <MenuItem key={opt} value={opt}>
                  {opt}
                </MenuItem>
              ))}
            </Select>
          </Collapse>
        </Grid>
        <Divider flexItem sx={{ width: '100%' }} />
        <Grid sx={{ flex: '1 1 auto', height: '1rem', overflow: 'auto' }}>
          <List>
            <Collapse in={!namesOnly}>
              <Subheader>Properties</Subheader>
              {Object.entries(PROPERTIES).map(([key, paramType]) => {
                const props = {
                  key,
                  item: key,
                  params,
                  setParams,
                }
                return {
                  boolean: <BoolItem {...props} />,
                  string: <TextItem {...props} type="text" />,
                  number: <TextItem {...props} type="number" />,
                }[paramType]
              })}
              <Divider sx={{ my: 2 }} />
            </Collapse>
            <Subheader>Params</Subheader>
            {Object.entries(PARAMS).map(([key, paramType]) => {
              const props = {
                key,
                item: key,
                params,
                setParams,
              }
              return {
                boolean: <BoolItem {...props} />,
                string: <TextItem {...props} type="text" />,
                number: <TextItem {...props} type="number" />,
              }[paramType]
            })}
          </List>
        </Grid>
        <Divider flexItem sx={{ width: '100%' }} />
        <Grid sx={{ flexGrow: 0 }}>
          <TextField
            value={url}
            disabled
            fullWidth
            InputProps={{
              endAdornment: (
                <IconButton
                  onPointerDown={() =>
                    navigator.clipboard.writeText(
                      `${window.location.host}${url}`,
                    )
                  }
                >
                  <ContentCopyIcon />
                </IconButton>
              ),
            }}
          />
        </Grid>
      </Grid>
      <Grid xs={11} sm={6} textAlign="left">
        <Code
          height="100vh"
          width="100%"
          code={error || code}
          setCode={setCode}
        />
      </Grid>
    </Grid>
  )
}
