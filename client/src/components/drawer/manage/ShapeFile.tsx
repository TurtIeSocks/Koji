/* eslint-disable no-console */
import { ListItemButton, ListItemIcon, ListItemText } from '@mui/material'
import * as React from 'react'
import * as shapefile from 'shapefile'
import UploadFileIcon from '@mui/icons-material/UploadFile'
import type { Feature } from 'geojson'
import { useShapes } from '@hooks/useShapes'
import { convert } from '@services/fetches'
import { usePersist } from '@hooks/usePersist'

export default function ShapeFile() {
  const [value, setValue] = React.useState<Feature[]>([])
  const [shpString, setShpString] = React.useState<string | ArrayBuffer>('')
  const [dbfString, setDbfString] = React.useState<string | ArrayBuffer>('')

  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)
  const add = useShapes((s) => s.setters.add)

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) {
      return
    }

    Array.from(e.target.files).forEach((file) => {
      const reader = new FileReader()
      reader.onload = (evt) => {
        if (!evt?.target?.result) {
          return
        }
        if (file.name.endsWith('.shp')) {
          setShpString(evt.target.result)
        }
        if (file.name.endsWith('.dbf')) {
          setDbfString(evt.target.result)
        }
      }
      reader.readAsArrayBuffer(file)
    })
  }

  React.useEffect(() => {
    if (shpString) {
      shapefile
        .open(shpString, dbfString || undefined)
        .then((source) =>
          source.read().then(function write(result): Promise<void> | void {
            if (result.done) return
            setValue((prev) => [...prev, result.value])
            return source.read().then(write)
          }),
        )
        .catch((error) => console.error(error.stack))
    }
  }, [shpString, dbfString])

  React.useEffect(() => {
    if (value.length) {
      convert<Feature[]>(
        value.map((feat) => ({
          ...feat,
          id: feat.properties?.town,
          properties: {
            ...feat.properties,
            name: feat.properties?.town,
            type: 'AutoQuest',
          },
        })),
        'featureVec',
        simplifyPolygons,
      ).then((geojson) => {
        add(geojson)
        setValue([])
      })
    }
  }, [value])

  return (
    <ListItemButton component="label" sx={{ marginRight: '1rem' }}>
      <ListItemIcon>
        <UploadFileIcon />
      </ListItemIcon>
      <ListItemText primary="ShapeFile" />
      <input
        type="file"
        hidden
        multiple
        accept=".shp,.dbf"
        onChange={handleFileUpload}
      />
    </ListItemButton>
  )
}
