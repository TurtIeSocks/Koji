import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import type { FeatureCollection } from 'geojson'

import {
  Box,
  Button,
  Chip,
  MenuItem,
  Select,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from '@mui/material'
import { useStatic, UseStatic } from '@hooks/useStatic'
import useSkipFirstEffect from '@hooks/useSkipFirstEffect'

const modifyName = (
  name: unknown,
  mod: UseStatic['importWizard']['modifier'],
): string => {
  if (typeof name === 'string') {
    switch (mod) {
      case 'capitalize':
        return name
          .split(' ')
          .map(
            (word) =>
              word.charAt(0).toUpperCase() + word.slice(1).toLowerCase(),
          )
          .join(' ')
      case 'lowercase':
        return name.toLowerCase()
      case 'uppercase':
        return name.toUpperCase()
      default:
        return name
    }
  }
  if (typeof name === 'number' || typeof name === 'boolean') {
    return name.toString()
  }
  if (Array.isArray(name)) {
    return name.join('')
  }
  if (typeof name === 'object') {
    return JSON.stringify(name)
  }
  return 'Invalid Property'
}

const PropsStep = React.forwardRef<
  HTMLDivElement,
  {
    handleChange: (geojson: FeatureCollection) => void
    geojson: FeatureCollection
  }
>(({ handleChange, geojson }, ref) => {
  const importWizard = useStatic((s) => s.importWizard)

  const [availableProps] = React.useState(() => {
    if (!geojson) return []
    const available = new Set<string>()
    geojson.features.forEach((feature) => {
      if (feature.properties) {
        Object.keys(feature.properties).forEach((key) => available.add(key))
      }
    })
    const filtered = Array.from(available).filter((key) =>
      geojson.features.every(
        (feat) => feat.properties?.[key] !== undefined && !key.startsWith('__'),
      ),
    )
    return filtered
  })

  useSkipFirstEffect(() => {
    handleChange({
      ...geojson,
      features: geojson.features.map((feat, i) => {
        const moddedName = modifyName(
          feat.properties?.[importWizard.nameProp] || `feature_${i}`,
          importWizard.modifier,
        )
        const name = (
          importWizard.customName
            ? `${importWizard.customName
                .replace(/{name}/g, moddedName)
                .replace(/{index}/g, i.toString())}${
                /{name}|{index}/g.test(importWizard.customName) ? '' : `_${i}`
              }`
            : moddedName
        )
          .trim()
          .replaceAll('\u0000', '')
        return {
          ...feat,
          properties: {
            ...Object.fromEntries(
              Object.entries(feat.properties ?? {}).map(([key, v]) => {
                const actualKey = key.startsWith('__') ? key.slice(2) : key
                return importWizard.props.includes(actualKey)
                  ? [actualKey, v]
                  : [`__${actualKey}`, v]
              }),
            ),
            name,
          },
        }
      }),
    })
  }, [
    importWizard.props.length,
    importWizard.nameProp,
    importWizard.customName,
    importWizard.modifier,
  ])

  React.useEffect(() => {
    if (availableProps.includes('name') && !importWizard.nameProp) {
      useStatic.setState({
        importWizard: { ...importWizard, nameProp: 'name' },
      })
    }
  }, [])

  return (
    <Grid2 container ref={ref} minHeight="30vh">
      <Grid2 xs={6} mt={3}>
        <Typography gutterBottom variant="h5">
          Feature Name
        </Typography>
      </Grid2>
      <Grid2 xs={6} mt={3}>
        <Typography gutterBottom variant="h5">
          Feature Properties
        </Typography>
      </Grid2>
      <Grid2
        xs={6}
        container
        p={3}
        justifyContent="space-between"
        minHeight="40vh"
      >
        <Grid2 xs={5}>
          <Typography gutterBottom>Property</Typography>
        </Grid2>
        <Grid2 xs={7}>
          <Select
            required
            fullWidth
            value={importWizard.nameProp}
            onChange={({ target }) => {
              useStatic.setState({
                importWizard: { ...importWizard, nameProp: target.value },
              })
            }}
          >
            {availableProps.map((prop) => (
              <MenuItem key={prop} value={prop}>
                {prop}
              </MenuItem>
            ))}
          </Select>
          <Typography variant="caption">
            *Properties found on all features
          </Typography>
        </Grid2>
        <Grid2 xs={5}>
          <Typography gutterBottom>Custom</Typography>
        </Grid2>
        <Grid2 xs={7}>
          <TextField
            label="Custom Name"
            value={importWizard.customName}
            fullWidth
            onChange={({ target }) =>
              useStatic.setState({
                importWizard: { ...importWizard, customName: target.value },
              })
            }
            helperText="Try using {name} and {index}"
          />
        </Grid2>
        <Grid2 xs={12}>
          <ToggleButtonGroup size="small" fullWidth sx={{ mx: 'auto' }}>
            {(['none', 'capitalize', 'uppercase', 'lowercase'] as const).map(
              (value) => (
                <ToggleButton
                  key={value}
                  value={value}
                  selected={importWizard.modifier === value}
                  onClick={() =>
                    useStatic.setState({
                      importWizard: { ...importWizard, modifier: value },
                    })
                  }
                >
                  {value}
                </ToggleButton>
              ),
            )}
          </ToggleButtonGroup>
        </Grid2>
      </Grid2>
      <Grid2 xs={6} p={3}>
        <Box
          sx={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: 0.5,
            border: 'darkgrey 2px solid',
            borderRadius: 5,
            p: 2,
          }}
        >
          {availableProps.map((value) => {
            const selected = importWizard.props.includes(value)
            if (value === 'name') return null
            return (
              <Chip
                key={value}
                label={value}
                clickable
                size="small"
                color={selected ? 'primary' : 'default'}
                deleteIcon={selected ? undefined : <div />}
                onClick={() => {
                  const values = selected
                    ? importWizard.props.filter((v) => v !== value)
                    : [...importWizard.props, value]
                  useStatic.setState({
                    importWizard: { ...importWizard, props: values },
                  })
                }}
                onDelete={() => {
                  useStatic.setState({
                    importWizard: {
                      ...importWizard,
                      props: importWizard.props.filter((v) => v !== value),
                    },
                  })
                }}
              />
            )
          })}
        </Box>
        <Button
          onClick={() => {
            useStatic.setState({
              importWizard: {
                ...importWizard,
                props: availableProps,
              },
            })
          }}
        >
          Select All
        </Button>
        <Button
          onClick={() => {
            useStatic.setState({
              importWizard: {
                ...importWizard,
                props: [],
              },
            })
          }}
        >
          Select None
        </Button>
      </Grid2>
    </Grid2>
  )
})

PropsStep.displayName = 'PropsStep'

export default PropsStep
