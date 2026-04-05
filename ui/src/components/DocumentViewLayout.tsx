import { useMemo, useRef, type ReactNode } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

import { Button } from 'primereact/button';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';
import { DocumentActions } from './DocumentActions';

type DocumentViewLayoutProps = {
  documentId: number;
  children: ReactNode;
};

export function DocumentViewLayout({ documentId, children }: Readonly<DocumentViewLayoutProps>) {
  const navigate = useNavigate();
  const location = useLocation();
  const menuRef = useRef<Menu>(null);
  const menuId = `document-view-menu-${documentId}`;

  const menuItems = useMemo<MenuItem[]>(
    () => {
      const items = [
        {
          label: 'Preview',
          icon: 'pi pi-eye',
          path: `/documents/${documentId}/preview`,
        },
        {
          label: 'Metadata',
          icon: 'pi pi-database',
          path: `/documents/${documentId}/metadata`,
        },
        {
          label: 'Indexes',
          icon: 'pi pi-list',
          path: `/documents/${documentId}/indexes`,
        },
        {
          label: 'Files',
          icon: 'pi pi-file',
          path: `/documents/${documentId}/files`,
        },
        {
          label: 'Document Text',
          icon: 'pi pi-align-left',
          path: `/documents/${documentId}/text-content`,
        },
        {
          label: 'Document OCR',
          icon: 'pi pi-search',
          path: `/documents/${documentId}/ocr-content`,
        },
      ];

      return items.map((item) => ({
        label: item.label,
        icon: item.icon,
        command: () => navigate(item.path),
        className: location.pathname === item.path ? 'aut-document-view-menu-active' : undefined,
      }));
    },
    [documentId, location.pathname, navigate]
  );

  return (
    <div className="aut-document-view-layout">
      <div className="aut-document-view-content">
        {children}
      </div>
      <aside className="aut-document-view-side">
        <DocumentActions
          documentIds={[documentId]}
          containerClassName="aut-document-view-actions"
        />
        <div className="aut-document-view-menu-mobile">
          <Menu
            model={menuItems}
            popup
            ref={menuRef}
            id={menuId}
            popupAlignment="right"
            className="aut-document-view-menu-list"
          />
          <Button
            icon="pi pi-bars"
            text
            onClick={(event) => menuRef.current?.toggle(event)}
            aria-controls={menuId}
            aria-haspopup
          />
        </div>
        <div className="aut-document-view-menu aut-document-view-menu-desktop">
          <Menu model={menuItems} className="aut-document-view-menu-list" />
        </div>
      </aside>
    </div>
  );
}
