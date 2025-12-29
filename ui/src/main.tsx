import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { createBrowserRouter, Outlet, RouterProvider } from 'react-router-dom';
import './index.scss'
import Navigation from './navigation.tsx';
import {
  QueryClient,
  QueryClientProvider,
} from '@tanstack/react-query'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'

import { PrimeReactProvider } from 'primereact/api';
import './App.scss'
import 'primereact/resources/themes/md-dark-indigo/theme.css';
import { ListCabinets } from './pages/cabinets.tsx';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

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
    ],
  },
]);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ReactQueryDevtools />
      <PrimeReactProvider>
        <RouterProvider router={router} />
      </PrimeReactProvider>
    </QueryClientProvider>
  </StrictMode>,
)
