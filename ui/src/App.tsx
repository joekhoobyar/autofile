import { PrimeReactProvider } from 'primereact/api';
import { ListCabinets } from './pages/cabinets';

import './App.scss'
import 'primereact/resources/themes/md-dark-indigo/theme.css';

export default function App() {
  return (
    <PrimeReactProvider>
      <main>
        <ListCabinets/>
      </main>
    </PrimeReactProvider>
  )
}
