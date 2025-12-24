// import { useState } from 'react'
import { useCabinets } from './queries/useCabinets'
import { type Cabinet } from './models/cabinet'

import './App.scss'

export default function App() {
  return (
    <ListCabinets/>
  )
}

function ListItemCabinet(cabinet: Cabinet) {
  return (
    <li>
      <header>{cabinet.name}</header>
      <p>{cabinet.description}</p>
    </li>
  )
}

function ListCabinets() {
  const { isPending, error, data, isFetching } = useCabinets();

  if (isPending) return 'Loading...'

  if (error) return 'An error has occurred: ' + error.message

  return (
    <ul className="model-list cabinets">
      {data.map((cabinet: Cabinet) => (
        <ListItemCabinet key={cabinet.id} {...cabinet} />
      ))}
      {isFetching ? 'Updating...' : null}
    </ul>
  )
}
