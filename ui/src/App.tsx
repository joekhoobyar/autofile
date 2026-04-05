// framework
import { useEffect, useRef, useState } from 'react';
import { createBrowserRouter, RouterProvider, NavLink, useLocation } from 'react-router-dom';
import { PrimeReactProvider } from 'primereact/api';
import 'primereact/resources/primereact.min.css';
import 'primereact/resources/themes/lara-dark-indigo/theme.css';
import 'primeicons/primeicons.css';
import 'primeflex/primeflex.css';
import { BreadCrumb } from 'primereact/breadcrumb';
        
// app
import './App.scss'
import UploadDocument, { ListDocuments } from './pages/documents.tsx';
import { EditCabinet, ListCabinets, NewCabinet } from './pages/cabinets.tsx';
import { EditMetadataType, ListMetadataTypes, NewMetadataType } from './pages/metadataTypes.tsx';
import { EditDocumentType, ListDocumentTypes, NewDocumentType } from './pages/documentTypes.tsx';
import { EditTag, ListTags, NewTag } from './pages/tags.tsx';
import { NAV, useBreadcrumbs } from './nav.ts';
import { Button } from 'primereact/button';
import Login, { Logout, RequireAuth } from './pages/auth.tsx';
import { AuthProvider } from './AuthProvider.tsx';
import { EditDocumentMetadata } from './pages/documentMetadata.tsx';
import { ListDocumentFilePageTextContent } from './pages/documentFiles.tsx';
import { EditDocumentIndex, ListDocumentIndexes, NewDocumentIndex } from './pages/documentIndexes.tsx';
import { EditDocumentIndexTemplate, ListDocumentIndexTemplates, NewDocumentIndexTemplate } from './pages/documentIndexTemplates.tsx';
import { ListDocumentIndexValues } from './pages/documentIndexValues.tsx';

export function SideNav() {
  const location = useLocation();

  return (
    <nav className="side-nav">
      {NAV.map(item => {
        const isSectionActive = item.matchPrefix
          ? location.pathname === item.to || location.pathname.startsWith(item.to + "/")
          : location.pathname === item.to;

        return (
          <NavLink
            key={item.key}
            to={item.to}
            className={({ isActive }) =>
              "side-nav-item " + ((isActive || isSectionActive) ? "is-active" : "")
            }
          >
            {item.icon && <i className={`${item.icon} mr-2`} />}
            <span>{item.label}</span>
          </NavLink>
        );
      })}
    </nav>
  );
}

function Layout() {
  const { home, model } = useBreadcrumbs();
  const { pathname }= useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const prevPathRef = useRef(pathname);

  useEffect(() => {
    if (prevPathRef.current !== pathname) {
      prevPathRef.current = pathname;
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setMobileOpen(false);
    }
  }, [pathname, mobileOpen]);

  return (
    <div className={`app-shell ${mobileOpen ? "nav-open" : ""}`}>
      <header className="app-topbar">
        <Button icon="pi pi-bars" text onClick={() => setMobileOpen(true)} />
        <div className="app-topbar-title">Autofile</div>
      </header>

      {/* Backdrop for mobile */}
      <div className="app-backdrop" onClick={() => setMobileOpen(false)} />

      <div className="app-body">
        <aside className="app-nav">
          <SideNav />
        </aside>

        <main className="app-main">
          <BreadCrumb home={home} model={model} />
          <div className="mt-3">
            <RequireAuth />
          </div>
        </main>
      </div>
    </div>
  );
}

const router = createBrowserRouter([
  {
    path: '/login', element: <Login />
  },
  {
    path: '/logout', element: <Logout />
  },
  {
    path: '/',
    element: <Layout />,
    children: [
      { path: 'documents', 
        children: [
          { index: true, element: <ListDocuments/> },
          { path: 'new', element: <UploadDocument/> },
          { path: ':id/metadata', element: <EditDocumentMetadata/> },
          { path: ':id/text-content', element: <ListDocumentFilePageTextContent/> },
        ]
      },
      { path: 'cabinets', 
        children: [
          { index: true, element: <ListCabinets/> },
          { path: 'new', element: <NewCabinet/> },
          { path: ':id/edit', element: <EditCabinet/> },
          { path: ':cabinetId/documents', element: <ListDocuments/> },
        ]
      },
      { path: 'indexes', 
        children: [
          { index: true, element: <ListDocumentIndexes/> },
          { path: 'new', element: <NewDocumentIndex/> },
          { path: ':id/edit', element: <EditDocumentIndex/> },
          { path: ':documentIndexId/templates', 
            children: [
              { index: true, element: <ListDocumentIndexTemplates/> },
              { path: 'new', element: <NewDocumentIndexTemplate/> },
              { path: ':id/edit', element: <EditDocumentIndexTemplate/> },
            ]
          },
          { path: ':documentIndexId/values',
            children: [
              { index: true, element: <ListDocumentIndexValues/> },
            ]
          },
          { path: ':documentIndexId/values/:documentIndexValueId',
            children: [
              { index: true, element: <ListDocumentIndexValues/> },
            ]
          },
          { path: ':documentIndexId/values/:documentIndexValueId/documents',
            children: [
              { index: true, element: <ListDocuments/> },
            ]
          },
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
      },
      { path: 'tags', 
        children: [
          { index: true, element: <ListTags/> },
          { path: 'new', element: <NewTag/> },
          { path: ':id/edit', element: <EditTag/> },
          { path: ':tagId/documents', element: <ListDocuments/> },
        ]
      }
    ],
  },
]);

export default function App() {
  return (
    <PrimeReactProvider>
      <AuthProvider>
        <RouterProvider router={router} />
      </AuthProvider>
    </PrimeReactProvider>
  )
}
