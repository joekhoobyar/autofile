// framework
import { createBrowserRouter, Outlet, RouterProvider } from 'react-router-dom';
import { PrimeReactProvider } from 'primereact/api';
import 'bootstrap/scss/bootstrap.scss';
import 'primereact/resources/themes/bootstrap4-dark-blue/theme.css';
import 'primereact/resources/primereact.min.css';
import 'primeicons/primeicons.css';
import 'primeflex/primeflex.css';
        
// app
import Navigation from './navigation.tsx';
import './App.scss'
import { ListCabinets } from './pages/cabinets.tsx';
import { EditMetadataType, ListMetadataTypes, NewMetadataType } from './pages/metadataTypes.tsx';
import { EditDocumentType, ListDocumentTypes, NewDocumentType } from './pages/documentTypes.tsx';

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
      { path: 'document-types', 
        children: [
          { index: true, element: <ListDocumentTypes/> },
          { path: 'new', element: <NewDocumentType/> },
          { path: ':id/edit', element: <EditDocumentType/> },
        ]
      },
      { path: 'metadata-types', 
        children: [
          { index: true, element: <ListMetadataTypes/> },
          { path: 'new', element: <NewMetadataType/> },
          { path: ':id/edit', element: <EditMetadataType/> },
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
