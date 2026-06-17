import { List } from '@mui/material'
import * as React from 'react'

import { useStatic } from '@hooks/useStatic'
import { useShapes } from '@hooks/useShapes'
import { safeParse } from '@services/utils'

import { Code } from '../Code'
import ListSubheader from '../styled/Subheader'

export default function GeojsonTab() {
  const geojson = useStatic((s) => s.geojson)
  const { setFromCollection } = useShapes.getState().setters

  return (
    <List dense>
      <ListSubheader disableGutters>GeoJSON Preview</ListSubheader>
      <Code
        code={JSON.stringify(geojson, null, 2)}
        setCode={(newCode) => {
          const parsed = safeParse<typeof geojson>(newCode)
          if (!parsed.error) {
            setFromCollection(parsed.value)
          }
        }}
        maxHeight="85vh"
      />
    </List>
  )
}
