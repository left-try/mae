import React from 'react';

export interface SettingsViewProps {
  themeMode: string;
  compactMode: boolean;
}

export function SettingsView(props: SettingsViewProps) {
  return (
    <section aria-label="Desktop Settings Screen">
      <h2>Desktop Preferences & Appearance</h2>
      <select aria-label="Theme Mode Select" onChange={(e) => setThemeMode(e.target.value)}>
        <option value="dark-slate">Dark Slate</option>
        <option value="light-fluent">Light Fluent</option>
        <option value="high-contrast">High Contrast</option>
      </select>
      <input
        type="range"
        aria-label="UI Scale Slider"
        min={100}
        max={150}
        onChange={(e) => setUiScalePercent(e.target.value)}
      />
      <input
        type="checkbox"
        aria-label="Compact Density Checkbox"
        checked={props.compactMode}
        onChange={() => setCompactMode(!props.compactMode)}
      />
      <input
        type="checkbox"
        aria-label="Hardware Acceleration Checkbox"
        onChange={() => setGpuAccel(true)}
      />
      <button aria-label="Save Preferences Button" onClick={() => setPreferencesSaved(true)}>
        Save Preferences
      </button>
      <button aria-label="Reset Defaults Button" onClick={() => setPreferencesReset(true)}>
        Reset to Defaults
      </button>
    </section>
  );
}
