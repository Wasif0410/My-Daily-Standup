import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsPanel } from "@/features/settings/SettingsPanel";
import type { Settings } from "@/types/settings";

function settings(overrides: Partial<Settings> = {}): Settings {
  return {
    priorityThreshold: 5,
    weekStartsOn: "monday",
    launchAtLogin: false,
    ...overrides,
  };
}

describe("SettingsPanel", () => {
  it("shows what is currently set, not a guess", () => {
    render(
      <SettingsPanel
        settings={settings({
          priorityThreshold: 7,
          weekStartsOn: "sunday",
          launchAtLogin: true,
        })}
        onChange={vi.fn()}
      />,
    );

    expect(screen.getByLabelText(/week starts/i)).toHaveValue("sunday");
    expect(screen.getByLabelText(/launch at login/i)).toBeChecked();
  });

  it("sends only the field that changed", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<SettingsPanel settings={settings()} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText(/week starts/i), "saturday");

    // An omitted field means "leave alone". Sending the whole object would
    // write back values the user never touched, and would overwrite a change
    // made in another window between load and save.
    expect(onChange).toHaveBeenCalledWith({ weekStartsOn: "saturday" });
  });

  it("no longer offers a priority cut-off", () => {
    // The General board shows every priority, so a threshold would be a
    // control that changes nothing.
    render(<SettingsPanel settings={settings()} onChange={vi.fn()} />);

    expect(screen.queryByLabelText(/priority/i)).not.toBeInTheDocument();
  });

  it("toggles launch at login", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<SettingsPanel settings={settings()} onChange={onChange} />);

    await user.click(screen.getByLabelText(/launch at login/i));

    expect(onChange).toHaveBeenCalledWith({ launchAtLogin: true });
  });

  it("surfaces a rejected change rather than failing silently", () => {
    render(
      <SettingsPanel
        settings={settings()}
        onChange={vi.fn()}
        error={{ kind: "invalid-input", message: "week must start on a known day" }}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(/known day/);
  });

  it("keeps showing the stored value when a change was rejected", () => {
    // Not optimistic: Rust validates and can refuse, and launch-at-login can
    // fail at the OS. The panel must never show a value the database did not
    // accept.
    render(
      <SettingsPanel
        settings={settings({ weekStartsOn: "sunday" })}
        onChange={vi.fn()}
        error={{ kind: "invalid-input", message: "nope" }}
      />,
    );

    expect(screen.getByLabelText(/week starts/i)).toHaveValue("sunday");
  });
});
