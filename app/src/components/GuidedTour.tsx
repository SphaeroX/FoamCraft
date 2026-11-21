// SPDX-License-Identifier: GPL-3.0-or-later
// A short guided tour through the four working steps.
import { useState } from "react";
import { useTranslation } from "../i18n/i18n";

const STEPS = ["geometry", "scenario", "run", "results"] as const;

export function GuidedTour({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const [i, setI] = useState(0);
  const step = STEPS[i];
  const last = i === STEPS.length - 1;
  return (
    <div className="dialog-backdrop" role="presentation">
      <div
        className="dialog panel tour-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tour-title"
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      >
        <p className="small muted">
          {t("tour.count", { n: String(i + 1), of: String(STEPS.length) })}
        </p>
        <h2 id="tour-title">{t(`tour.${step}.title`)}</h2>
        <p>{t(`tour.${step}.text`)}</p>
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>
            {t("tour.skip")}
          </button>
          {i > 0 && (
            <button type="button" onClick={() => setI(i - 1)}>
              {t("tour.back")}
            </button>
          )}
          <button type="button" autoFocus onClick={() => (last ? onClose() : setI(i + 1))}>
            {last ? t("tour.done") : t("tour.next")}
          </button>
        </div>
      </div>
    </div>
  );
}
