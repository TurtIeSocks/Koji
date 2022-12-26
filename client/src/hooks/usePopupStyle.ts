import { type Theme, useTheme } from '@mui/material'
import { useEffect } from 'react'
import { useMapEvent } from 'react-leaflet'

function applyPopups(color: string) {
  const popups = document.getElementsByClassName(
    'leaflet-popup-content-wrapper',
  )
  for (let i = 0; i < popups.length; i++) {
    popups[i].setAttribute('style', `background-color: ${color} !important`)
  }
  const tips = document.getElementsByClassName('leaflet-popup-tip')
  for (let i = 0; i < tips.length; i++) {
    tips[i].setAttribute('style', `background-color: ${color} !important`)
  }
}

export default function usePopupStyle(theme?: Theme) {
  const safeTheme = theme || useTheme()

  useMapEvent('popupopen', () => {
    if (safeTheme) applyPopups(safeTheme.palette.background.paper)
  })

  useEffect(() => {
    if (safeTheme) applyPopups(safeTheme.palette.background.paper)
  }, [safeTheme?.palette.mode])
}
