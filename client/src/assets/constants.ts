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
  v: `<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="deeppink" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
      </svg>`,
  u: `<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="dodgerblue" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
      </svg>`,
  p: `<svg xmlns="http://www.w3.org/2000/svg" height="20" width="20" viewBox="-2 -2 24 24">
        <circle cx="10" cy="10" r="10" fill="green" fill-opacity="0.8" stroke="black" stroke-width="1" />
        <circle cx="10" cy="10" r="1" fill="black" />
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
} as const

export const ICON_COLOR = {
  p: 'green',
  g: 'maroon',
  v: 'deeppink',
  u: 'dodgerblue',
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
  'AutoPokemon',
  'AutoQuest',
  'AutoTth',
  'PokemonIV',
] as const

export const RDM_ROUTES = [
  'CirclePokemon',
  'CircleSmartPokemon',
  'CircleRaid',
  'CircleSmartRaid',
] as const

export const UNOWN_FENCES = ['AutoQuest'] as const

export const UNOWN_ROUTES = [
  'CirclePokemon',
  'CircleRaid',
  'ManualQuest',
] as const

export const COLORS = [
  '#F0F8FF',
  '#FAEBD7',
  '#00FFFF',
  '#7FFFD4',
  '#F0FFFF',
  '#F5F5DC',
  '#FFE4C4',
  '#000000',
  '#FFEBCD',
  '#0000FF',
  '#8A2BE2',
  '#A52A2A',
  '#DEB887',
  '#5F9EA0',
  '#7FFF00',
  '#D2691E',
  '#FF7F50',
  '#6495ED',
  '#FFF8DC',
  '#DC143C',
  '#00FFFF',
  '#00008B',
  '#008B8B',
  '#B8860B',
  '#A9A9A9',
  '#006400',
  '#A9A9A9',
  '#BDB76B',
  '#8B008B',
  '#556B2F',
  '#FF8C00',
  '#9932CC',
  '#8B0000',
  '#E9967A',
  '#8FBC8F',
  '#483D8B',
  '#2F4F4F',
  '#2F4F4F',
  '#00CED1',
  '#9400D3',
  '#FF1493',
  '#00BFFF',
  '#696969',
  '#1E90FF',
  '#B22222',
  '#FFFAF0',
  '#228B22',
  '#FF00FF',
  '#DCDCDC',
  '#F8F8FF',
  '#FFD700',
  '#DAA520',
  '#808080',
  '#008000',
  '#ADFF2F',
  '#808080',
  '#F0FFF0',
  '#FF69B4',
  '#CD5C5C',
  '#4B0082',
  '#FFFFF0',
  '#F0E68C',
  '#E6E6FA',
  '#FFF0F5',
  '#7CFC00',
  '#FFFACD',
  '#ADD8E6',
  '#F08080',
  '#E0FFFF',
  '#FAFAD2',
  '#D3D3D3',
  '#90EE90',
  '#D3D3D3',
  '#FFB6C1',
  '#FFA07A',
  '#20B2AA',
  '#87CEFA',
  '#778899',
  '#778899',
  '#B0C4DE',
  '#FFFFE0',
  '#00FF00',
  '#32CD32',
  '#FAF0E6',
  '#FF00FF',
  '#800000',
  '#66CDAA',
  '#0000CD',
  '#BA55D3',
  '#9370DB',
  '#3CB371',
  '#7B68EE',
  '#00FA9A',
  '#48D1CC',
  '#C71585',
  '#191970',
  '#F5FFFA',
  '#FFE4E1',
  '#FFE4B5',
  '#FFDEAD',
  '#000080',
  '#FDF5E6',
  '#808000',
  '#6B8E23',
  '#FFA500',
  '#FF4500',
  '#DA70D6',
  '#EEE8AA',
  '#98FB98',
  '#AFEEEE',
  '#DB7093',
  '#FFEFD5',
  '#FFDAB9',
  '#CD853F',
  '#FFC0CB',
  '#DDA0DD',
  '#B0E0E6',
  '#800080',
  '#663399',
  '#FF0000',
  '#BC8F8F',
  '#4169E1',
  '#8B4513',
  '#FA8072',
  '#F4A460',
  '#2E8B57',
  '#FFF5EE',
  '#A0522D',
  '#C0C0C0',
  '#87CEEB',
  '#6A5ACD',
  '#708090',
  '#708090',
  '#FFFAFA',
  '#00FF7F',
  '#4682B4',
  '#D2B48C',
  '#008080',
  '#D8BFD8',
  '#FF6347',
  '#40E0D0',
  '#EE82EE',
  '#F5DEB3',
  '#FFFFFF',
  '#F5F5F5',
  '#FFFF00',
  '#9ACD32',
]
