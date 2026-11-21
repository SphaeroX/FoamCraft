// SPDX-License-Identifier: GPL-3.0-or-later
import { isLanguage, languages, useTranslation } from "../i18n/i18n";

export function LanguageSwitcher() {
  const { language, setLanguage, t } = useTranslation();
  return (
    <label className="language-switcher">
      <span className="visually-hidden">{t("toolbar.language")}</span>
      <select
        aria-label={t("toolbar.language")}
        value={language}
        onChange={(e) => {
          if (isLanguage(e.target.value)) setLanguage(e.target.value);
        }}
      >
        {languages.map((lang) => (
          <option key={lang} value={lang}>
            {t(`language.${lang}`)}
          </option>
        ))}
      </select>
    </label>
  );
}
