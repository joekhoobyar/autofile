// framework
import { useEffect, useRef, useState } from 'react';
import { createBrowserRouter, Navigate, RouterProvider, NavLink, useLocation } from 'react-router-dom';
import { PrimeReactProvider } from 'primereact/api';
import 'primereact/resources/primereact.min.css';
import 'primereact/resources/themes/lara-dark-indigo/theme.css';
import 'primeicons/primeicons.css';
import 'primeflex/primeflex.css';
import { BreadCrumb } from 'primereact/breadcrumb';
        
// app
import './App.scss'
import UploadDocument, { EditDocumentProperties, ListDocuments } from './pages/documents.tsx';
import { EditCabinet, ListCabinets, NewCabinet } from './pages/cabinets.tsx';
import { EditMetadataType, ListMetadataTypes, NewMetadataType } from './pages/metadataTypes.tsx';
import { EditDocumentType, ListDocumentTypes, NewDocumentType } from './pages/documentTypes.tsx';
import { EditTag, ListTags, NewTag } from './pages/tags.tsx';
import { NAV, useBreadcrumbs } from './nav.ts';
import { Button } from 'primereact/button';
import Login, { Logout, RequireAdmin, RequireAuth } from './pages/auth.tsx';
import { AuthProvider } from './AuthProvider.tsx';
import { canManageUsers, useAuth } from './auth.ts';
import { EditDocumentMetadata } from './pages/documentMetadata.tsx';
import { DocumentFilePagePreview, ListDocumentFilePageOcrContent, ListDocumentFilePageTextContent, ListDocumentFiles, UploadDocumentFile } from './pages/documentFiles.tsx';
import { ListDocumentIndexMembership } from './pages/documentIndexMembership.tsx';
import { EditDocumentIndex, ListDocumentIndexes, NewDocumentIndex } from './pages/documentIndexes.tsx';
import { EditDocumentIndexTemplate, ListDocumentIndexTemplates, NewDocumentIndexTemplate } from './pages/documentIndexTemplates.tsx';
import { ListDocumentIndexValues } from './pages/documentIndexValues.tsx';
import { EditClassifierBlock, ListClassifierBlocks, NewClassifierBlock } from './pages/classifierBlocks.tsx';
import { DocumentClassifierTest } from './pages/documentClassifierTest.tsx';
import { DocumentTemplateTest } from './pages/documentTemplateTest.tsx';
import { EditUser, ListUsers, ViewUser } from './pages/users.tsx';

export function SideNav() {
  const auth = useAuth();
  const location = useLocation();
  const navItems = NAV.filter((item) => item.key !== 'users' || canManageUsers(auth));
  const isLogoutActive = location.pathname === '/logout';

  return (
    <nav className="side-nav">
      <div className="side-nav-main">
        {navItems.map(item => {
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
      </div>
      <NavLink
        to="/logout"
        className={`side-nav-item side-nav-logout ${isLogoutActive ? 'is-active' : ''}`}
      >
        Logout
      </NavLink>
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
      { index: true, element: <Navigate to="/documents" replace /> },
      { path: 'documents', 
        children: [
          { index: true, element: <ListDocuments/> },
          { path: 'new', element: <UploadDocument/> },
          { path: ':id/properties', element: <EditDocumentProperties/> },
          { path: ':id/metadata', element: <EditDocumentMetadata/> },
          { path: ':id/indexes', element: <ListDocumentIndexMembership/> },
          { path: ':id/files', element: <ListDocumentFiles/> },
          { path: ':id/files/new', element: <UploadDocumentFile/> },
          { path: ':id/text-content', element: <ListDocumentFilePageTextContent/> },
          { path: ':id/ocr-content', element: <ListDocumentFilePageOcrContent/> },
          { path: ':id/preview', element: <DocumentFilePagePreview/> },
          { path: ':id/classifier-test', element: <DocumentClassifierTest/> },
          { path: ':id/template-test', element: <DocumentTemplateTest/> },
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
      { path: 'classifier-blocks',
        children: [
          { index: true, element: <ListClassifierBlocks/> },
          { path: 'new', element: <NewClassifierBlock/> },
          { path: ':id/edit', element: <EditClassifierBlock/> },
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
      },
      { path: 'users',
        element: <RequireAdmin />,
        children: [
          { index: true, element: <ListUsers/> },
          { path: ':id', element: <ViewUser/> },
          { path: ':id/edit', element: <EditUser/> },
        ],
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
