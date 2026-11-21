// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { LanguageSwitcher } from "../../app/src/components/LanguageSwitcher";
import { LanguageProvider, STORAGE_KEY, useTranslation } from "../../app/src/i18n/i18n";

function Probe() {
  const { t } = useTranslation();
  return <p data-testid="probe">{t("toolbar.about")}</p>;
}

function renderUi() {
  return render(
    <LanguageProvider>
      <LanguageSwitcher />
      <Probe />
    </LanguageProvider>,
  );
}

describe("language switcher", () => {
  beforeEach(() => localStorage.clear());
  afterEach(cleanup);

  it("changes the visible text", () => {
    localStorage.setItem(STORAGE_KEY, "en");
    renderUi();
    expect(screen.getByTestId("probe").textContent).toBe("About");
    act(() => {
      fireEvent.change(screen.getByRole("combobox"), { target: { value: "de" } });
    });
    expect(screen.getByTestId("probe").textContent).toBe("Über");
  });

  it("stores the choice and restores it on the next start", () => {
    localStorage.setItem(STORAGE_KEY, "en");
    const first = renderUi();
    act(() => {
      fireEvent.change(screen.getByRole("combobox"), { target: { value: "de" } });
    });
    expect(localStorage.getItem(STORAGE_KEY)).toBe("de");
    first.unmount();
    renderUi();
    expect(screen.getByTestId("probe").textContent).toBe("Über");
  });
});
