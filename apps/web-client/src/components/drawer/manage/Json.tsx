/* eslint-disable no-console */
import * as React from 'react'
import { Button, CircularProgress } from '@mui/material'
import UploadFileIcon from '@mui/icons-material/UploadFile'

import type { FeatureCollection } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { usePersist } from '@hooks/usePersist'
import { convert } from '@services/fetches'

interface Props {
  setter?: (featureCollection: FeatureCollection) => void
}

export default function JsonFile({ setter }: Props) {
  const add = useShapes((s) => s.setters.add)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  const [fileName, setFileName] = React.useState<string>('')
  const [loading, setLoading] = React.useState<boolean>(false)

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) {
      return
    }
    setFileName(e.target.files[0].name)
    const reader = new FileReader()
    reader.onload = async function parse(newSettings) {
      if (newSettings?.target) {
        const contents = newSettings.target.result
        if (typeof contents === 'string') {
          setLoading(true)
          const parsed = JSON.parse(contents)
          console.log('JSON File Results:', parsed)
          const geojson = await convert<FeatureCollection>(
            parsed,
            'featureCollection',
            simplifyPolygons,
          ).then((geo) => {
            setLoading(false)
            return geo
          })
          if (geojson.type === 'FeatureCollection') {
            if (setter) {
              setter(geojson)
            } else {
              add(geojson.features)
            }
          }
        }
      }
    }
    reader.readAsText(e.target.files[0])
  }

  return loading ? (
    <CircularProgress color="secondary" />
  ) : (
    <Button
      variant="contained"
      component="label"
      color="secondary"
      sx={{ maxWidth: '90%' }}
      endIcon={fileName ? undefined : <UploadFileIcon />}
    >
      {fileName
        ? fileName.length > 15
          ? `${fileName.substring(0, 15)}...`
          : fileName
        : 'Browse'}
      <input
        type="file"
        hidden
        accept=".json, .geojson"
        onChange={handleFileUpload}
      />
    </Button>
  )
}
