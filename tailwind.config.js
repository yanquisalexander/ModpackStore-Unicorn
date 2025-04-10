const withMT = require("@material-tailwind/react/utils/withMT");
import colors from "tailwindcss/colors";

/** @type {import('tailwindcss').Config} */
module.exports = withMT({
  content: [
    "./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"
  ],
  purge: [],
  darkMode: false, // or 'media' or 'class'
  theme: {
    colors,
    extend: {
    },
  },
  variants: {
    extend: {},
  },
  plugins: [],
})
