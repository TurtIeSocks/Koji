/* eslint-disable no-console */
import { ListItemButton, ListItemIcon, ListItemText } from '@mui/material'
import * as React from 'react'
import UploadFileIcon from '@mui/icons-material/UploadFile'

import { useShapes } from '@hooks/useShapes'
import { convert } from '@services/fetches'
import type { FeatureCollection } from 'geojson'
import { usePersist } from '@hooks/usePersist'

export default function JsonFile() {
  const add = useShapes((s) => s.setters.add)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) {
      return
    }
    const reader = new FileReader()
    reader.onload = async function parse(newSettings) {
      if (newSettings?.target) {
        const contents = newSettings.target.result
        if (typeof contents === 'string') {
          const geojson = await convert<FeatureCollection>(
            JSON.parse(contents),
            'featureCollection',
            simplifyPolygons,
          )
          if (geojson.type === 'FeatureCollection') {
            add(geojson.features)
          }
        }
      }
    }
    reader.readAsText(e.target.files[0])
  }

  return (
    <ListItemButton component="label" sx={{ marginRight: '1rem' }}>
      <ListItemIcon>
        <UploadFileIcon />
      </ListItemIcon>
      <ListItemText primary="JSON File" />
      <input
        type="file"
        hidden
        accept=".json, .geojson"
        onChange={handleFileUpload}
      />
    </ListItemButton>
  )
}
