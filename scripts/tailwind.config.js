/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        stellar: {
          50: '#f0f4ff',
          100: '#e0eaff',
          500: '#3b82f6',
          600: '#2563eb',
          900: '#0f172a',
        },
        soroban: {
          50: '#fdf4ff',
          500: '#a855f7',
          600: '#9333ea',
        }
      }
    },
  },
  plugins: [],
}
