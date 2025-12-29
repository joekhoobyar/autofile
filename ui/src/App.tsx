// framework
import { createBrowserRouter, Outlet, RouterProvider } from 'react-router-dom';
import { PrimeReactProvider } from 'primereact/api';
import 'primereact/resources/themes/md-dark-indigo/theme.css';

// app
import Navigation from './navigation.tsx';
import './App.scss'
import { ListCabinets } from './pages/cabinets.tsx';
import { ListMetadataTypes } from './pages/metadataTypes.tsx';

export function Layout() {
  return (
    <>
      <Navigation />
      <Outlet />
    </>
  );
}

const router = createBrowserRouter([
  {
    path: '/',
    element: <Layout />,
    children: [
      { index: true, element: <ListCabinets/> },
      { path: 'cabinets', 
        children: [
          { index: true, element: <ListCabinets/> },
        ]
      },
      { path: 'metadata-types', 
        children: [
          { index: true, element: <ListMetadataTypes/> },
        ]
      }
    ],
  },
]);

export default function App() {
  return (
    <PrimeReactProvider>
      <RouterProvider router={router} />
    </PrimeReactProvider>
  )
}
