import { useState } from "react";

import { Button } from "primereact/button";
import { Card } from "primereact/card";
import { Dialog } from "primereact/dialog";
import { Message } from "primereact/message";

import { DescriptionList } from "../components/DescriptionList";
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
        <DescriptionList
          items={[
            { label: "Application", value: "Autofile" },
            { label: "Version", value: data.version },
            { label: "Backend Package", value: data.name },
            { label: "Author", value: data.authors },
            { label: "Copyright", value: data.copyright },
            {
              label: "License",
              value: (
                <span className="aut-description-list-status">
                  <span>{data.license}</span>
                  <Button
                    label="View License"
                    icon="pi pi-file"
                    size="small"
                    text
                    onClick={() => setLicenseVisible(true)}
                  />
                </span>
              ),
            },
          ]}
        />
      </Card>

      <Dialog
        header="Autofile License"
        visible={licenseVisible}
        onHide={() => setLicenseVisible(false)}
        style={{ width: "min(760px, 95vw)" }}
      >
        {license.isError && <Message severity="error" text={license.error.message} />}
        {license.isLoading && <div>Loading</div>}
        {license.data && <pre className="aut-license-text">{license.data}</pre>}
      </Dialog>
    </>
  );
}
