/*
  p - pokestop
  g - gym
  v - verified spawnpoint
  u - unverified spawnpoint

  Shortened to reduce fetch sizes
*/
import Map from '@mui/icons-material/Map'
import EditRoad from '@mui/icons-material/EditRoad'
import ImportExport from '@mui/icons-material/ImportExport'
import Settings from '@mui/icons-material/Settings'
import TravelExplore from '@mui/icons-material/TravelExplore'
import Layers from '@mui/icons-material/Layers'

export const ATTRIBUTION = `
  <a href='https://github.com/TurtIeSocks/Koji' noreferrer='true' target='_blank'>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 18" width="12" height="12">
      <path d="M12 1.27a11 11 0 00-3.48 21.46c.55.09.73-.28.73-.55v-1.84c-3.03.64-3.67-1.46-3.67-1.46-.55-1.29-1.28-1.65-1.28-1.65-.92-.65.1-.65.1-.65 1.1 0 1.73 1.1 1.73 1.1.92 1.65 2.57 1.2 3.21.92a2 2 0 01.64-1.47c-2.47-.27-5.04-1.19-5.04-5.5 0-1.1.46-2.1 1.2-2.84a3.76 3.76 0 010-2.93s.91-.28 3.11 1.1c1.8-.49 3.7-.49 5.5 0 2.1-1.38 3.02-1.1 3.02-1.1a3.76 3.76 0 010 2.93c.83.74 1.2 1.74 1.2 2.94 0 4.21-2.57 5.13-5.04 5.4.45.37.82.92.82 2.02v3.03c0 .27.1.64.73.55A11 11 0 0012 1.27"/>
    </svg>
    Kōji - TurtleSocks
  </a>
`

export const ICON_SVG = {
  u: `<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="deeppink" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
      </svg>`,
  v: `<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="dodgerblue" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
      </svg>`,
  p: `<svg xmlns="http://www.w3.org/2000/svg" height="20" width="20" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="green" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
      </svg>`,
  r: `<svg xmlns="http://www.w3.org/2000/svg" height="78" width="78" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="darkgreen" fill-opacity="0.2" stroke="darkgreen" stroke-width="0.25" />
      </svg>`,
  g: `<svg xmlns="http://www.w3.org/2000/svg" width="25" height="25" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="maroon" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
      </svg>`,
} as const

export const ICON_RADIUS = {
  p: 15,
  g: 20,
  v: 10,
  u: 10,
  r: 70,
} as const

export const ICON_COLOR = {
  p: 'green',
  g: 'maroon',
  v: 'deeppink',
  u: 'dodgerblue',
  r: 'darkgreen',
} as const

export const TABS = [
  'Drawing',
  'Clustering',
  'Layers',
  'Manage',
  'Geojson',
  'Settings',
] as const

export const ICON_MAP: Record<typeof TABS[number], typeof EditRoad> = {
  Drawing: EditRoad,
  Layers,
  Clustering: TravelExplore,
  Manage: ImportExport,
  Settings,
  Geojson: Map,
}

export const RDM_FENCES = [
  'auto_pokemon',
  'auto_quest',
  'auto_tth',
  'pokemon_iv',
] as const

export const RDM_ROUTES = [
  'circle_pokemon',
  'circle_smart_pokemon',
  'circle_raid',
  'circle_smart_raid',
] as const

export const UNOWN_FENCES = ['auto_quest'] as const

export const UNOWN_ROUTES = [
  'circle_pokemon',
  'circle_raid',
  'circle_quest',
] as const

export const ALL_FENCES = [...new Set([...RDM_FENCES, ...UNOWN_FENCES])]

export const ALL_ROUTES = [...new Set([...RDM_ROUTES, ...UNOWN_ROUTES])]

export const CONVERSION_TYPES = [
  'array',
  'multiArray',
  'geometry',
  'geometry_vec',
  'feature',
  'feature_vec',
  'featureCollection',
  'struct',
  'multiStruct',
  'text',
  'altText',
  'poracle',
] as const

export const GEOMETRY_CONVERSION_TYPES = [
  'Point',
  'MultiPoint',
  'Polygon',
  'MultiPolygon',
] as const

export const PROPERTY_CATEGORIES = [
  'boolean',
  'string',
  'number',
  'object',
  'array',
  'database',
  'color',
] as const

export const S2_CELL_LEVELS = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]

export const KEYBOARD_SHORTCUTS = [
  {
    category: 'drawing',
    shortcuts: [
      'cut',
      'drag',
      'drawCircle',
      'drawPolygon',
      'drawRectangle',
      'edit',
      // 'merge',
      'remove',
      'rotate',
    ],
  },
  {
    category: 'shapes',
    shortcuts: ['arrows', 'circles', 'lines', 'polygons'],
  },
  {
    category: 'data',
    shortcuts: ['gyms', 'pokestops', 'spawnpoints'],
  },
  {
    category: 'other',
    shortcuts: ['drawer', 'setTileServer', 'theme'],
  },
]
