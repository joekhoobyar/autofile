import { useState } from "react";

import { Button } from "primereact/button";
import { Card } from "primereact/card";
import { Dialog } from "primereact/dialog";
import { Message } from "primereact/message";

import { useAppInfo, useAppLicense } from "../queries/useAppInfo";

export function About() {
  const [licenseVisible, setLicenseVisible] = useState(false);
  const { data, isError, isLoading, error } = useAppInfo();
  const license = useAppLicense(licenseVisible);

  if (isError) {
    return <Message severity="error" text={error.message} />;
  }

  if (isLoading || !data) {
    return <div>Loading</div>;
  }

  return (
    <>
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
          <li className="aut-about-license-row">
            <span>License</span>: {data.license}
            <Button
              label="View License"
              icon="pi pi-file"
              size="small"
              text
              onClick={() => setLicenseVisible(true)}
            />
          </li>
          <li>
            <span>Copyright</span>: {data.copyright}
          </li>
        </ul>
      </Card>

      <Dialog
        header="Autofile License"
        visible={licenseVisible}
        onHide={() => setLicenseVisible(false)}
        style={{ width: "min(900px, 95vw)" }}
      >
        {license.isError && <Message severity="error" text={license.error.message} />}
        {license.isLoading && <div>Loading</div>}
        {license.data && <pre className="aut-license-text">{license.data}</pre>}
      </Dialog>
    </>
  );
}
