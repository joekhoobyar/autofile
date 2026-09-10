import { Badge } from 'primereact/badge';

import type { Tag } from '../models/tag';

type DocumentTagBadgeProps = {
  tag: Pick<Tag, 'name' | 'color'>;
};

export function DocumentTagBadge({ tag }: Readonly<DocumentTagBadgeProps>) {
  return (
    <Badge
      value={tag.name}
      className="aut-document-tag"
      style={{ backgroundColor: `#${tag.color}` }}
    />
  );
}
