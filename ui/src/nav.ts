import { useMemo } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import type { MenuItem } from "primereact/menuitem";

import { useDocument } from "./queries/useDocuments";
import { useCabinet } from "./queries/useCabinets";
import { useDocumentType } from "./queries/useDocumentTypes";
import { useMetadataType } from "./queries/useMetadataTypes";
import { useTag } from "./queries/useTags";

export type NavItem = {
  key: string;
  label: string;
  icon?: string;       // PrimeIcons class, e.g. "pi pi-folder"
  to: string;          // absolute path recommended
  matchPrefix?: boolean; // keep section active for subroutes
};

export const NAV: NavItem[] = [
  { key: "documents", label: "Documents", icon: "pi pi-file", to: "/documents", matchPrefix: true },
  { key: "cabinets", label: "Cabinets", icon: "pi pi-inbox", to: "/cabinets", matchPrefix: true },
  { key: "document-types", label: "Document Types", icon: "pi pi-file", to: "/document-types", matchPrefix: true },
  { key: "metadata-types", label: "Metadata Types", icon: "pi pi-list", to: "/metadata-types", matchPrefix: true },
  { key: "tags", label: "Tags", icon: "pi pi-tags", to: "/tags", matchPrefix: true },
];

// useRouteResourceLabel.ts

type LabelState = { label?: string; loading?: boolean };

export function useRouteResourceLabel(): LabelState {
  const { id } = useParams<{ id: string }>();
  const { pathname } = useLocation();

  const inDocuments = pathname.startsWith("/documents/");
  const inCabinets = pathname.startsWith("/cabinets/");
  const inDocTypes = pathname.startsWith("/document-types/");
  const inMetaTypes = pathname.startsWith("/metadata-types/");
  const inTags = pathname.startsWith("/tags/");

  // Call *one* query hook based on route; keep others disabled
  const docQ = useDocument(id!, { enabled: !!id && inDocuments });
  const cabinetQ = useCabinet(id!, { enabled: !!id && inCabinets });
  const docTypeQ = useDocumentType(id!, { enabled: !!id && inDocTypes });
  const metaTypeQ = useMetadataType(id!, { enabled: !!id && inMetaTypes });
  const tagQ = useTag(id!, { enabled: !!id && inMetaTypes });

  if (!id) return {};

  if (inDocuments) return { label: docQ.data?.title, loading: docQ.isLoading };
  if (inCabinets) return { label: cabinetQ.data?.name, loading: cabinetQ.isLoading };
  if (inDocTypes) return { label: docTypeQ.data?.name, loading: docTypeQ.isLoading };
  if (inMetaTypes) return { label: metaTypeQ.data?.name, loading: metaTypeQ.isLoading };
  if (inTags) return { label: tagQ.data?.name, loading: tagQ.isLoading };

  return {};
}


export function useBreadcrumbs(): { home: MenuItem; model: MenuItem[] } {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { id } = useParams<{ id: string }>();
  const resource = useRouteResourceLabel();

  const home: MenuItem = useMemo(
    () => ({ icon: "pi pi-home", command: () => navigate("/") }),
    [navigate]
  );

  const model = useMemo(() => {
    const section = NAV.find(s => pathname === s.to || pathname.startsWith(s.to + "/"));
    if (!section) return [];

    const items: MenuItem[] = [{ label: section.label, command: () => navigate(section.to) }];

    if (pathname.endsWith("/new")) {
      items.push({ label: "New" });
      return items;
    }

    if (pathname.endsWith("/edit")) {
      if (id) {
        const label = resource.loading ? "Loading…" : (resource.label ?? id);
        items.push({ label });
      }
      items.push({ label: "Edit" });
      return items;
    }

    return items;
  }, [pathname, navigate, id, resource.loading, resource.label]);

  return { home, model };
}