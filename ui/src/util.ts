import { useParams } from "react-router-dom";

export function useId(name: string): number {
  const params = useParams();
  const value = params[name];

  if (!value) {
    throw new Error(`Missing route param: ${name}`);
  }

  const id = Number(value)
  if (Number.isNaN(id)) {
    throw new TypeError(`Invalid route param: ${name}`);
  }

  return id;
}
