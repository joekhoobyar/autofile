import { useMemo } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import type { MenuItem } from "primereact/menuitem";

import { useDocument } from "./queries/useDocuments";
import { useCabinet } from "./queries/useCabinets";
import { useDocumentType } from "./queries/useDocumentTypes";
import { useMetadataType } from "./queries/useMetadataTypes";
import { useTag } from "./queries/useTags";
import { useDocumentIndex } from "./queries/useDocumentIndexes";
import { useDocumentIndexTemplate } from "./queries/useDocumentIndexTemplates";
import { useClassifierBlock } from "./queries/useClassifierBlocks";
import { useUser } from "./queries/useUsers";

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
  { key: "classifier-blocks", label: "Classifiers", icon: "pi pi-sitemap", to: "/classifier-blocks", matchPrefix: true },
  { key: "indexes", label: "Indexes", icon: "pi pi-list", to: "/indexes", matchPrefix: true },
  { key: "document-types", label: "Document Types", icon: "pi pi-file", to: "/document-types", matchPrefix: true },
  { key: "metadata-types", label: "Metadata Types", icon: "pi pi-database", to: "/metadata-types", matchPrefix: true },
  { key: "tags", label: "Tags", icon: "pi pi-tags", to: "/tags", matchPrefix: true },
  { key: "users", label: "Users", icon: "pi pi-users", to: "/users", matchPrefix: true },
];

// useRouteResourceLabel.ts

type LabelState = { label?: string; loading?: boolean };

const DOCUMENT_VIEW_ROUTE_LABELS: Record<string, string> = {
  properties: 'Properties',
  preview: 'Preview',
  metadata: 'Metadata',
  indexes: 'Indexes',
  files: 'Files',
  'text-content': 'Document Text',
  'ocr-content': 'Document OCR',
};

export function useRouteResourceLabel(): LabelState {
  const { id, documentIndexId } = useParams<{ id: string; documentIndexId: string }>();
  const { pathname } = useLocation();
  const documentIndexIdNum = documentIndexId ? Number(documentIndexId) : NaN;

  const inDocuments = pathname.startsWith("/documents/");
  const inCabinets = pathname.startsWith("/cabinets/");
  const inClassifierBlocks = pathname.startsWith("/classifier-blocks/");
  const inIndexTemplates = pathname.includes("/indexes/") && pathname.includes("/templates") 
  const inIndexes = pathname.startsWith("/indexes/") && !inIndexTemplates;
  const inDocTypes = pathname.startsWith("/document-types/");
  const inMetaTypes = pathname.startsWith("/metadata-types/");
  const inTags = pathname.startsWith("/tags/");
  const inUsers = pathname.startsWith("/users/");

  // Call *one* query hook based on route; keep others disabled
  const docQ = useDocument(id!, { enabled: !!id && inDocuments });
  const cabinetQ = useCabinet(id!, { enabled: !!id && inCabinets });
  const classifierBlockQ = useClassifierBlock(id!, { enabled: !!id && inClassifierBlocks });
  const indexQ = useDocumentIndex(id!, { enabled: !!id && inIndexes });
  const indexTemplateQ = useDocumentIndexTemplate(documentIndexIdNum, id!, {
    enabled: !!id && !!documentIndexId && !Number.isNaN(documentIndexIdNum) && inIndexTemplates,
  });
  const docTypeQ = useDocumentType(id!, { enabled: !!id && inDocTypes });
  const metaTypeQ = useMetadataType(id!, { enabled: !!id && inMetaTypes });
  const tagQ = useTag(id!, { enabled: !!id && inTags });
  const userQ = useUser(id!, { enabled: !!id && inUsers });

  if (!id) return {};

  if (inDocuments) return { label: docQ.data?.title, loading: docQ.isLoading };
  if (inCabinets) return { label: cabinetQ.data?.name, loading: cabinetQ.isLoading };
  if (inClassifierBlocks) return { label: classifierBlockQ.data?.name, loading: classifierBlockQ.isLoading };
  if (inIndexTemplates) return { label: indexTemplateQ.data?.template, loading: indexTemplateQ.isLoading };
  if (inIndexes) return { label: indexQ.data?.name, loading: indexQ.isLoading };
  if (inDocTypes) return { label: docTypeQ.data?.name, loading: docTypeQ.isLoading };
  if (inMetaTypes) return { label: metaTypeQ.data?.name, loading: metaTypeQ.isLoading };
  if (inTags) return { label: tagQ.data?.name, loading: tagQ.isLoading };
  if (inUsers) return { label: userQ.data?.username, loading: userQ.isLoading };

  return {};
}


export function useBreadcrumbs(): { home: MenuItem; model: MenuItem[] } {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { id, documentIndexId, tagId, cabinetId } = useParams<{ id: string; documentIndexId: string; tagId: string; cabinetId: string }>();
  const documentIndexIdNum = documentIndexId ? Number(documentIndexId) : NaN;
  const resource = useRouteResourceLabel();
  const inIndexTemplates = pathname.includes("/indexes/") && pathname.includes("/templates");
  const documentDetailMatch = pathname.match(/^\/documents\/([^/]+)\/([^/]+)$/);
  const documentDetailId = documentDetailMatch?.[1];
  const documentDetailSection = documentDetailMatch?.[2];
  const cabinetDocumentsMatch = pathname.match(/^\/cabinets\/([^/]+)\/documents$/);
  const tagDocumentsMatch = pathname.match(/^\/tags\/([^/]+)\/documents$/);
  const indexQ = useDocumentIndex(documentIndexIdNum, {
    enabled: !!documentIndexId && !Number.isNaN(documentIndexIdNum) && inIndexTemplates,
  });
  const cabinetQ = useCabinet(cabinetId ?? '', {
    enabled: !!cabinetId && !!cabinetDocumentsMatch,
  });
  const tagQ = useTag(tagId ?? '', {
    enabled: !!tagId && !!tagDocumentsMatch,
  });

  const home: MenuItem = useMemo(
    () => ({ icon: "pi pi-home", command: () => navigate("/") }),
    [navigate]
  );

  const model = useMemo(() => {
    const section = NAV.find(s => pathname === s.to || pathname.startsWith(s.to + "/"));
    if (!section) return [];

    const items: MenuItem[] = [{ label: section.label, command: () => navigate(section.to) }];

    if (inIndexTemplates && documentIndexId && !Number.isNaN(documentIndexIdNum)) {
      const indexLabel = indexQ.isLoading ? "Loading…" : (indexQ.data?.name ?? documentIndexId);
      items.push({ label: indexLabel, command: () => navigate(`/indexes/${documentIndexId}/edit`) });
      items.push({ label: "Templates", command: () => navigate(`/indexes/${documentIndexId}/templates`) });
    }

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

    if (cabinetDocumentsMatch && cabinetId) {
      const cabinetLabel = cabinetQ.isLoading ? 'Loading…' : (cabinetQ.data?.name ?? cabinetId);
      items.push({ label: cabinetLabel, command: () => navigate(`/cabinets/${cabinetId}/edit`) });
      items.push({ label: 'Documents' });
      return items;
    }

    if (tagDocumentsMatch && tagId) {
      const tagLabel = tagQ.isLoading ? 'Loading…' : (tagQ.data?.name ?? tagId);
      items.push({ label: tagLabel, command: () => navigate(`/tags/${tagId}/edit`) });
      items.push({ label: 'Documents' });
      return items;
    }

    if (documentDetailId && documentDetailSection && documentDetailSection in DOCUMENT_VIEW_ROUTE_LABELS) {
      const label = resource.loading ? 'Loading…' : (resource.label ?? documentDetailId);
      items.push({ label, command: () => navigate(`/documents/${documentDetailId}/preview`) });
      items.push({ label: DOCUMENT_VIEW_ROUTE_LABELS[documentDetailSection] });
      return items;
    }

    const userDetailMatch = pathname.match(/^\/users\/([^/]+)$/);
    if (userDetailMatch && id) {
      const label = resource.loading ? 'Loading…' : (resource.label ?? id);
      items.push({ label });
      return items;
    }

    return items;
  }, [
    pathname,
    navigate,
    id,
    resource.loading,
    resource.label,
    inIndexTemplates,
    documentIndexId,
    documentIndexIdNum,
    indexQ.isLoading,
    indexQ.data?.name,
    cabinetId,
    cabinetDocumentsMatch,
    cabinetQ.isLoading,
    cabinetQ.data?.name,
    tagId,
    tagDocumentsMatch,
    tagQ.isLoading,
    tagQ.data?.name,
    documentDetailId,
    documentDetailSection,
  ]);

  return { home, model };
}
