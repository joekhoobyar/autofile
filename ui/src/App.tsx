// import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'

import './App.scss'

interface Cabinet {
  id: number
  slug: string
  name: string
  description: string
}

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
  const { isPending, error, data, isFetching } = useQuery({
    queryKey: ['cabinets', 'list'],
    queryFn: async () => {
      const response = await fetch(
        'http://localhost:8000/cabinets',
      )
      return await response.json()
    },
  })

  if (isPending) return 'Loading...'

  if (error) return 'An error has occurred: ' + error.message

  return (
    <ul>
      {data.map((cabinet: Cabinet) => (
        <ListItemCabinet key={cabinet.id} {...cabinet} />
      ))}
      {isFetching ? 'Updating...' : null}
    </ul>
  )
}
