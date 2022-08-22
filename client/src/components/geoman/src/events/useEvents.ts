import { useEffect } from 'react'

import drawEvents from './draw'
import type { HandlersWithFallback } from '../types'
import editEvents from './edit'
import cutEvents from './cut'
import dragEvents from './drag'
import removeEvents from './remove'
import rotateEvents from './rotate'

export default function useEvents(
  hasMounted: boolean,
  handlers: HandlersWithFallback,
  map: L.Map | null,
) {
  useEffect(() => {
    if (hasMounted && map) {
      cutEvents(map, handlers, 'on')
      dragEvents(map, handlers, 'on')
      drawEvents(map, handlers, 'on')
      editEvents(map, handlers, 'on')
      removeEvents(map, handlers, 'on')
      rotateEvents(map, handlers, 'on')
      return () => {
        cutEvents(map, handlers, 'off')
        dragEvents(map, handlers, 'off')
        drawEvents(map, handlers, 'off')
        editEvents(map, handlers, 'off')
        removeEvents(map, handlers, 'off')
        rotateEvents(map, handlers, 'off')
      }
    }
  }, [hasMounted, !map, Object.values(handlers).every((h) => !h)])
}
