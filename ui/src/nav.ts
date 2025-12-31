import { useMemo } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";

import type { MenuItem } from "primereact/menuitem";

export type NavItem = {
  key: string;
  label: string;
  icon?: string;       // PrimeIcons class, e.g. "pi pi-folder"
  to: string;          // absolute path recommended
  matchPrefix?: boolean; // keep section active for subroutes
};

export const NAV: NavItem[] = [
  { key: "cabinets", label: "Cabinets", icon: "pi pi-inbox", to: "/cabinets", matchPrefix: true },
  { key: "document-types", label: "Document Types", icon: "pi pi-file", to: "/document-types", matchPrefix: true },
  { key: "metadata-types", label: "Metadata Types", icon: "pi pi-tags", to: "/metadata-types", matchPrefix: true },
];

export function useBreadcrumbs(): { home: MenuItem; model: MenuItem[] } {
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams(); // { id?: string } etc.

  const home: MenuItem = useMemo(
    () => ({ icon: "pi pi-home", command: () => navigate("/") }),
    [navigate]
  );

  const model = useMemo(() => {
    const path = location.pathname;

    // Find which top-level section we are in
    const section = NAV.find(s => path === s.to || path.startsWith(s.to + "/"));

    if (!section) return [];

    const items: MenuItem[] = [
      { label: section.label, command: () => navigate(section.to) },
    ];

    // Simple CRUD-ish crumbs based on suffix
    if (path.endsWith("/new")) {
      items.push({ label: "New" });
    } else if (path.endsWith("/edit")) {
      // optional: insert resource id/name crumb
      if (params.id) items.push({ label: params.id }); // replace w/ fetched name if desired
      items.push({ label: "Edit" });
    }

    return items;
  }, [location.pathname, navigate, params.id]);

  return { home, model };
}