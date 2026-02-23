import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: "class",

  content: ["./index.html", "./src/**/*.{ts,tsx}"],

  theme: {
    extend: {
      colors: {
        // Neutral palette (background, surfaces, text)
        neutral: {
          50: "#fafaf9",
          100: "#f5f5f4",
          200: "#e7e5e4",
          300: "#d6d3d1",
          400: "#a8a29e",
          500: "#78716c",
          600: "#57534e",
          700: "#44403c",
          800: "#292524",
          850: "#1f1d1c",
          900: "#1c1917",
          950: "#0c0a09",
        },

        // 12 curated accent colors for Space theming
        accent: {
          amber: "#f59e0b",
          orange: "#f97316",
          rose: "#f43f5e",
          pink: "#ec4899",
          violet: "#8b5cf6",
          indigo: "#6366f1",
          blue: "#3b82f6",
          cyan: "#06b6d4",
          teal: "#14b8a6",
          emerald: "#10b981",
          lime: "#84cc16",
          sand: "#d4a574",
        },

        // Semantic role colors
        role: {
          host: "#d4a017", // Gold for Host
          creator: "#3b82f6", // Blue for Creator
          moderator: "#8b5cf6", // Purple for Moderator
          member: "#78716c", // Neutral for regular members
        },

        // Application semantic colors
        surface: {
          primary: "var(--surface-primary)",
          secondary: "var(--surface-secondary)",
          tertiary: "var(--surface-tertiary)",
          overlay: "var(--surface-overlay)",
        },

        border: {
          DEFAULT: "var(--border-default)",
          subtle: "var(--border-subtle)",
        },

        text: {
          primary: "var(--text-primary)",
          secondary: "var(--text-secondary)",
          tertiary: "var(--text-tertiary)",
          inverse: "var(--text-inverse)",
        },
      },

      fontFamily: {
        sans: [
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          '"Segoe UI"',
          "Roboto",
          '"Helvetica Neue"',
          "Arial",
          '"Noto Sans"',
          "sans-serif",
          '"Apple Color Emoji"',
          '"Segoe UI Emoji"',
          '"Segoe UI Symbol"',
          '"Noto Color Emoji"',
        ],
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          '"Liberation Mono"',
          '"Courier New"',
          "monospace",
        ],
      },

      // Spacing for the sidebar / layout grid
      spacing: {
        "sidebar": "16rem",
        "sidebar-collapsed": "4rem",
      },

      // Border radius tokens
      borderRadius: {
        DEFAULT: "0.5rem",
        sm: "0.25rem",
        md: "0.5rem",
        lg: "0.75rem",
        xl: "1rem",
      },

      // Animation timing for Framer Motion consistency
      transitionDuration: {
        DEFAULT: "200ms",
        fast: "100ms",
        normal: "200ms",
        slow: "300ms",
      },
    },
  },

  plugins: [],
};

export default config;
