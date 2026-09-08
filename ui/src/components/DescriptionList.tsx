import type { ReactNode } from "react";

export type DescriptionListItem = {
  label: string;
  value: ReactNode;
  fullWidth?: boolean;
};

export function DescriptionList({ items }: Readonly<{ items: DescriptionListItem[] }>) {
  return (
    <dl className="aut-description-list">
      {items.map((item) => (
        <div key={item.label} className={item.fullWidth ? "aut-description-list-row is-full-width" : "aut-description-list-row"}>
          <dt>{item.label}</dt>
          <dd>{item.value}</dd>
        </div>
      ))}
    </dl>
  );
}
