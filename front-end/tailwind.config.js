/** @type {import('tailwindcss').Config} */

/** A colour from a design token in src/index.css, usable with opacity (`bg-accent/20`). */
const token = (name) => `rgb(var(--${name}) / <alpha-value>)`;

export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // The neutral scale every surface, line and text colour is drawn from.
        studio: {
          950: token("studio-950"),
          900: token("studio-900"),
          850: token("studio-850"),
          800: token("studio-800"),
          700: token("studio-700"),
          600: token("studio-600"),
          500: token("studio-500"),
          400: token("studio-400"),
          300: token("studio-300"),
          200: token("studio-200"),
          100: token("studio-100"),
        },
        // Roles: one meaning per colour, everywhere.
        accent: {
          DEFAULT: token("accent"),
          hover: token("accent-hover"),
          fg: token("accent-fg"),
        },
        video: { DEFAULT: token("video"), fg: token("video-fg") },
        audio: { DEFAULT: token("audio"), fg: token("audio-fg") },
        caption: { DEFAULT: token("caption"), fill: token("caption-fill") },
        zoom: { DEFAULT: token("zoom"), fg: token("zoom-fg") },
        suggest: { DEFAULT: token("suggest"), fg: token("suggest-fg") },
        danger: { DEFAULT: token("danger"), fg: token("danger-fg") },
        success: { DEFAULT: token("success"), fg: token("success-fg") },
      },
      fontFamily: {
        sans: [
          '"Segoe UI Variable Text"',
          '"Segoe UI"',
          "-apple-system",
          "BlinkMacSystemFont",
          "Roboto",
          '"Helvetica Neue"',
          "Arial",
          "sans-serif",
        ],
        display: ['"Segoe UI Variable Display"', '"Segoe UI"', "-apple-system", "sans-serif"],
        mono: ["ui-monospace", "Cascadia Mono", "Consolas", "SFMono-Regular", "Menlo", "monospace"],
      },
      // The type scale. Nothing smaller than `meta` (12px).
      fontSize: {
        meta: ["12px", { lineHeight: "16px" }],
        label: ["13px", { lineHeight: "18px" }],
        body: ["14px", { lineHeight: "20px" }],
        reading: ["16px", { lineHeight: "26px" }],
        title: ["16px", { lineHeight: "22px", fontWeight: "600" }],
        heading: ["20px", { lineHeight: "28px", fontWeight: "600" }],
        display: ["28px", { lineHeight: "36px", fontWeight: "600" }],
      },
      // Control heights: buttons and fields line up across the app.
      height: {
        "control-sm": "28px",
        control: "32px",
        "control-lg": "36px",
        header: "48px",
        statusbar: "28px",
      },
      minHeight: {
        "control-sm": "28px",
        control: "32px",
        "control-lg": "36px",
      },
      borderRadius: {
        control: "6px",
        panel: "10px",
      },
      boxShadow: {
        popover: "0 12px 32px rgb(0 0 0 / 0.45), 0 2px 8px rgb(0 0 0 / 0.3)",
        dialog: "0 24px 64px rgb(0 0 0 / 0.55)",
      },
    },
  },
  plugins: [],
}
