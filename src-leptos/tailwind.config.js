/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./app/src/**/*.rs", "./style/**/*.css"],
  darkMode: "class",
  theme: {
    extend: {
      // ── Colour tokens ────────────────────────────────────────────────────
      colors: {
        // Surfaces
        "background":                "#f9f9ff",
        "surface":                   "#f9f9ff",
        "surface-dim":               "#d3daef",
        "surface-bright":            "#f9f9ff",
        "surface-container-lowest":  "#ffffff",
        "surface-container-low":     "#f1f3ff",
        "surface-container":         "#e9edff",
        "surface-container-high":    "#e1e8fd",
        "surface-container-highest": "#dce2f7",
        "surface-variant":           "#dce2f7",
        "surface-tint":              "#1357c9",

        // On-surface
        "on-background":      "#141b2b",
        "on-surface":         "#141b2b",
        "on-surface-variant": "#434653",

        // Inverse
        "inverse-surface":    "#293040",
        "inverse-on-surface": "#edf0ff",
        "inverse-primary":    "#b1c5ff",

        // Outline
        "outline":         "#737785",
        "outline-variant": "#c3c6d6",

        // Primary
        "primary":           "#003b93",
        "on-primary":        "#ffffff",
        "primary-container": "#0051c3",
        "on-primary-container": "#beceff",
        "primary-fixed":         "#dae2ff",
        "primary-fixed-dim":     "#b1c5ff",
        "on-primary-fixed":          "#001947",
        "on-primary-fixed-variant":  "#00419f",

        // Secondary
        "secondary":              "#515f74",
        "on-secondary":           "#ffffff",
        "secondary-container":    "#d5e3fc",
        "on-secondary-container": "#57657a",
        "secondary-fixed":            "#d5e3fc",
        "secondary-fixed-dim":        "#b9c7df",
        "on-secondary-fixed":         "#0d1c2e",
        "on-secondary-fixed-variant": "#3a485b",

        // Tertiary
        "tertiary":              "#772600",
        "on-tertiary":           "#ffffff",
        "tertiary-container":    "#9e3500",
        "on-tertiary-container": "#ffc1ab",
        "tertiary-fixed":            "#ffdbcf",
        "tertiary-fixed-dim":        "#ffb59a",
        "on-tertiary-fixed":         "#380d00",
        "on-tertiary-fixed-variant": "#802a00",

        // Error
        "error":              "#ba1a1a",
        "on-error":           "#ffffff",
        "error-container":    "#ffdad6",
        "on-error-container": "#93000a",

        // Status accents (utility)
        "success":    "#059669",
        "warning":    "#d97706",
        "code-bg":    "#111827",
      },

      // ── Border radius ────────────────────────────────────────────────────
      borderRadius: {
        "sm":      "0.125rem",
        DEFAULT:   "0.25rem",
        "md":      "0.375rem",
        "lg":      "0.5rem",
        "xl":      "0.75rem",
        "full":    "9999px",
      },

      // ── Spacing (4px base grid) ──────────────────────────────────────────
      spacing: {
        "unit": "4px",
        "xs":   "4px",
        "sm":   "8px",
        "md":   "16px",
        "lg":   "24px",
        "xl":   "32px",
        "container-margin": "32px",
        "gutter":           "16px",
      },

      // ── Typography ───────────────────────────────────────────────────────
      fontFamily: {
        sans:  ["Inter", "sans-serif"],
        mono:  ["JetBrains Mono", "monospace"],
      },
      fontSize: {
        "h1":         ["24px", { lineHeight: "32px",  letterSpacing: "-0.02em", fontWeight: "600" }],
        "h2":         ["20px", { lineHeight: "28px",  letterSpacing: "-0.01em", fontWeight: "600" }],
        "h3":         ["16px", { lineHeight: "24px",  fontWeight: "600" }],
        "body-md":    ["14px", { lineHeight: "20px",  fontWeight: "400" }],
        "body-sm":    ["13px", { lineHeight: "18px",  fontWeight: "400" }],
        "label-caps": ["11px", { lineHeight: "16px",  letterSpacing: "0.05em", fontWeight: "700" }],
        "mono":       ["13px", { lineHeight: "20px",  fontWeight: "400" }],
      },

      // ── Box shadow ───────────────────────────────────────────────────────
      boxShadow: {
        "card": "0px 1px 3px rgba(0,0,0,0.05)",
        "sm":   "0 1px 2px rgba(0,0,0,0.04)",
      },
    },
  },
  plugins: [
    require("@tailwindcss/forms"),
  ],
};
