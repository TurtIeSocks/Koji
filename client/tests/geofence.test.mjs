import assert from 'node:assert/strict'
import test from 'node:test'

import { getKojiGeofenceId } from '../src/services/geofence.ts'

const scanner = { id: 1, name: 'Test area', mode: 'auto_quest' }
const fence = { id: 20, name: 'Test area', mode: 'unset' }
const route = {
  id: 14,
  geofence_id: 20,
  name: 'Test area',
  mode: 'circle_pokemon',
}

test('a Koji fence keeps its own ID', () => {
  assert.equal(getKojiGeofenceId('20__unset__KOJI', fence, route, {}), 20)
})

test('a scanner fence uses the existing Koji route association', () => {
  assert.equal(
    getKojiGeofenceId('1__auto_quest__SCANNER', scanner, route, {}),
    20,
  )
})

test('a scanner fence without a route resolves the Koji fence by name', () => {
  assert.equal(
    getKojiGeofenceId('1__auto_quest__SCANNER', scanner, null, { 20: fence }),
    20,
  )
})

test('a matching numeric scanner ID is never used for an unrelated Koji fence', () => {
  assert.equal(
    getKojiGeofenceId('1__auto_quest__SCANNER', scanner, null, {
      1: { id: 1, name: 'Another area', mode: 'unset' },
    }),
    undefined,
  )
})

test('a scanner fence can resolve by name when route metadata has no geofence ID', () => {
  assert.equal(
    getKojiGeofenceId(
      '1__auto_quest__SCANNER',
      scanner,
      { id: 14, name: 'Test area', mode: 'circle_pokemon' },
      { 20: fence },
    ),
    20,
  )
})

test('a client-only fence without a cached reference leaves the ID unset', () => {
  assert.equal(
    getKojiGeofenceId('1__unset__CLIENT', null, null, { 20: fence }),
    undefined,
  )
})
