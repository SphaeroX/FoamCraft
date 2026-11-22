// SPDX-License-Identifier: GPL-3.0-or-later
// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AppMenu } from "../../app/src/components/AppMenu";
import { StepRail } from "../../app/src/components/StepRail";
import { LanguageProvider, STORAGE_KEY } from "../../app/src/i18n/i18n";
import { ExpertOnly, MODE_KEY, ModeProvider, ModeSwitch, useMode } from "../../app/src/mode";
import { canAdvance, stepStates } from "../../app/src/steps";

const empty = { models: 0, watertight: null, scenario: false, running: false, hasResult: false };

describe("step states", () => {
  it("starts with only the geometry open", () => {
    const s = stepStates(empty);
    expect(s.geometry).toEqual({ state: "open", missing: "missing.model" });
    expect(s.scenario.state).toBe("open");
    expect(s.run.state).toBe("open");
    expect(s.results.state).toBe("open");
    expect(canAdvance(s, "geometry")).toBe(false);
  });

  it("walks through the steps as the project fills", () => {
    let s = stepStates({ ...empty, models: 1, watertight: true });
    expect(s.geometry.state).toBe("done");
    expect(s.scenario).toEqual({ state: "ready", missing: "missing.scenario" });
    expect(canAdvance(s, "geometry")).toBe(true);
    expect(canAdvance(s, "scenario")).toBe(false);
    s = stepStates({ ...empty, models: 1, watertight: true, scenario: true });
    expect(s.scenario.state).toBe("done");
    expect(s.run).toEqual({ state: "ready", missing: "missing.run" });
    s = stepStates({ ...empty, models: 1, watertight: true, scenario: true, hasResult: true });
    expect(s.run.state).toBe("done");
    expect(s.results.state).toBe("ready");
    expect(canAdvance(s, "run")).toBe(true);
  });

  it("warns about an open model but lets the user go on", () => {
    const s = stepStates({ ...empty, models: 1, watertight: false });
    expect(s.geometry.state).toBe("attention");
    expect(canAdvance(s, "geometry")).toBe(true);
  });
});

describe("step rail and footer", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("shows each step with its state and the current one", () => {
    const states = stepStates({ ...empty, models: 1, watertight: true });
    const onStep = vi.fn();
    render(
      <LanguageProvider>
        <StepRail step="scenario" states={states} onStep={onStep} />
      </LanguageProvider>,
    );
    const geometry = screen.getByRole("button", { name: /Geometry/ });
    expect(geometry).toHaveAttribute("data-state", "done");
    expect(screen.getByRole("button", { name: /Scenario/ })).toHaveAttribute(
      "aria-current",
      "step",
    );
    fireEvent.click(geometry);
    expect(onStep).toHaveBeenCalledWith("geometry");
  });
});

describe("simple and expert", () => {
  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem(STORAGE_KEY, "en");
  });
  afterEach(cleanup);

  function Probe() {
    const { mode } = useMode();
    return <p data-testid="mode">{mode}</p>;
  }

  it("without a provider everything is shown (expert)", () => {
    render(
      <ExpertOnly>
        <button type="button">Add group</button>
      </ExpertOnly>,
    );
    expect(screen.getByRole("button", { name: "Add group" })).toBeInTheDocument();
  });

  it("the app starts simple, hides expert tools and remembers the choice", () => {
    const ui = () =>
      render(
        <LanguageProvider>
          <ModeProvider>
            <ModeSwitch />
            <Probe />
            <ExpertOnly>
              <button type="button">Add group</button>
            </ExpertOnly>
          </ModeProvider>
        </LanguageProvider>,
      );
    const first = ui();
    expect(screen.getByTestId("mode")).toHaveTextContent("simple");
    expect(screen.queryByRole("button", { name: "Add group" })).toBeNull();
    act(() => {
      fireEvent.click(screen.getByRole("checkbox", { name: "Expert mode" }));
    });
    expect(screen.getByRole("button", { name: "Add group" })).toBeInTheDocument();
    expect(localStorage.getItem(MODE_KEY)).toBe("expert");
    first.unmount();
    ui();
    expect(screen.getByTestId("mode")).toHaveTextContent("expert");
  });
});

describe("app menu", () => {
  beforeEach(() => localStorage.setItem(STORAGE_KEY, "en"));
  afterEach(cleanup);

  it("holds language, system, agents and about, but no tour", () => {
    const actions = {
      onSystem: vi.fn(),
      onAgents: vi.fn(),
      onAbout: vi.fn(),
    };
    render(
      <LanguageProvider>
        <AppMenu {...actions} />
      </LanguageProvider>,
    );
    const toggle = screen.getByRole("button", { name: "Menu" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(toggle);
    expect(screen.getByRole("combobox", { name: "Language" })).toBeInTheDocument();
    for (const [name, fn] of [
      ["System", actions.onSystem],
      ["Agents", actions.onAgents],
      ["About", actions.onAbout],
    ] as const) {
      fireEvent.click(screen.getByRole("menuitem", { name }));
      expect(fn).toHaveBeenCalled();
      fireEvent.click(toggle);
    }
    expect(screen.queryByRole("menuitem", { name: "Tour" })).toBeNull();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });
});
