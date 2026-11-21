// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { useTranslation } from "../i18n/i18n";
import { appInfo } from "../native";

export function AboutDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  useEffect(() => {
    appInfo()
      .then((info) => setVersion(info.version))
      .catch(() => setVersion("?"));
  }, []);
  return (
    <div className="dialog-backdrop" role="presentation" onClick={onClose}>
      <div
        className="dialog panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="about-title">{t("about.title")}</h2>
        <p className="muted">{t("about.version", { version })}</p>
        <p>{t("about.tagline")}</p>
        <p>{t("about.license")}</p>
        <p>{t("about.uses")}</p>
        <p className="small muted">{t("about.trademark")}</p>
        <div className="dialog-actions">
          <button type="button" onClick={onClose} autoFocus>
            {t("about.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
