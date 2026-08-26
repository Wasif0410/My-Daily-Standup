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

    expect(screen.getByLabelText(/priority/i)).toHaveValue("7");
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

  it("offers every priority from 0 to 10", () => {
    render(<SettingsPanel settings={settings()} onChange={vi.fn()} />);

    const options = screen.getByLabelText(/priority/i).querySelectorAll("option");
    expect([...options].map((o) => o.textContent?.trim())).toHaveLength(11);
  });

  it("says what the priority threshold actually does", () => {
    render(<SettingsPanel settings={settings()} onChange={vi.fn()} />);

    // The setting that most needs explaining: below it, a task simply is not
    // on the Priority board, which reads as the task having vanished. The
    // label alone does not say that, so the hint has to.
    expect(screen.getByText(/never appear on the Priority board/i)).toBeInTheDocument();
    expect(screen.getByText(/still on\s+Weekly Tasks/i)).toBeInTheDocument();
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
        error={{ kind: "invalid-input", message: "priority must be between 0 and 10" }}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(/between 0 and 10/);
  });

  it("keeps showing the stored value when a change was rejected", () => {
    // Not optimistic: Rust validates and can refuse, and launch-at-login can
    // fail at the OS. The panel must never show a value the database did not
    // accept.
    render(
      <SettingsPanel
        settings={settings({ priorityThreshold: 5 })}
        onChange={vi.fn()}
        error={{ kind: "invalid-input", message: "nope" }}
      />,
    );

    expect(screen.getByLabelText(/priority/i)).toHaveValue("5");
  });
});
