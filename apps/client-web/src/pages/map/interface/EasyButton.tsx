import * as React from 'react'
import * as L from 'leaflet'
import 'leaflet-easybutton/src/easy-button.css'
import 'leaflet-easybutton/src/easy-button'
import { useMap } from 'react-leaflet'

export default function EasyButton(options: L.EasyButtonOptions) {
  const map = useMap()
  const easyButton = React.useRef(L.easyButton(options))

  React.useEffect(() => {
    if (easyButton.current) {
      easyButton.current.addTo(map)
      return () => {
        easyButton.current.remove()
      }
    }
  }, [options])

  return null
}
