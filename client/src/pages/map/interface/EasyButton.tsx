import * as React from 'react'
import * as L from 'leaflet'
import 'leaflet-easybutton/src/easy-button.css'
import 'leaflet-easybutton/src/easy-button'
import { useMap } from 'react-leaflet'

export default function EasyButton(options: L.EasyButtonOptions) {
  const map = useMap()
  const [easyButton] = React.useState(L.easyButton(options))

  React.useEffect(() => {
    easyButton.addTo(map)
    return () => {
      easyButton.remove()
    }
  }, [])

  return null
}
