import { Card } from "primereact/card";
import { Message } from "primereact/message";

import { useAppInfo } from "../queries/useAppInfo";

export function About() {
  const { data, isError, isLoading, error } = useAppInfo();

  if (isError) {
    return <Message severity="error" text={error.message} />;
  }

  if (isLoading || !data) {
    return <div>Loading</div>;
  }

  return (
    <Card title="About Autofile">
      <ul className="aut-about-details">
        <li>
          <span>Application</span>: Autofile
        </li>
        <li>
          <span>Version</span>: {data.version}
        </li>
        <li>
          <span>Backend Package</span>: {data.name}
        </li>
        <li>
          <span>Author</span>: {data.authors}
        </li>
        <li>
          <span>License</span>: {data.license}
        </li>
        <li>
          <span>Copyright</span>: {data.copyright}
        </li>
      </ul>
    </Card>
  );
}
