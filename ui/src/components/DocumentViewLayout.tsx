import { useMemo, useRef, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';

import { Button } from 'primereact/button';
import { Menu } from 'primereact/menu';
import type { MenuItem } from 'primereact/menuitem';

type DocumentViewLayoutProps = {
  documentId: number;
  children: ReactNode;
};

export function DocumentViewLayout({ documentId, children }: DocumentViewLayoutProps) {
  const navigate = useNavigate();
  const menuRef = useRef<Menu>(null);
  const menuId = `document-view-menu-${documentId}`;

  const menuItems = useMemo<MenuItem[]>(
    () => [
      {
        label: 'Metadata',
        icon: 'pi pi-database',
        command: () => navigate(`/documents/${documentId}/metadata`),
      },
      {
        label: 'Document Text',
        icon: 'pi pi-align-left',
        command: () => navigate(`/documents/${documentId}/text-content`),
      },
    ],
    [documentId, navigate]
  );

  return (
    <div className="aut-document-view-layout">
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
      <div className="aut-document-view-content">
        {children}
      </div>
      <div className="aut-document-view-menu aut-document-view-menu-desktop">
        <Menu model={menuItems} className="aut-document-view-menu-list" />
      </div>
    </div>
  );
}
