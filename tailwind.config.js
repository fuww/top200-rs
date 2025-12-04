/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./templates/**/*.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {
      colors: {
        // Custom colors matching the existing chart palette
        'chart-emerald': 'rgb(16, 185, 129)',
        'chart-rose': 'rgb(244, 63, 94)',
        'chart-blue': 'rgb(59, 130, 246)',
      },
    },
  },
  plugins: [],
}
