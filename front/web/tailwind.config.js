const colors = require('tailwindcss/colors')

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{rs,html}",
    "./index.html",
  ],
  theme: {
    extend: {
      colors: {
        // Material Design colors
        primary: {
          50: '#e3f2fd',
          100: '#bbdefb',
          200: '#90caf9',
          300: '#64b5f6',
          400: '#42a5f5',
          500: '#2196f3', // Primary color
          600: '#1e88e5',
          700: '#1976d2',
          800: '#1565c0',
          900: '#0d47a1',
        },
        secondary: {
          50: '#fce4ec',
          100: '#f8bbd0',
          200: '#f48fb1',
          300: '#f06292',
          400: '#ec407a',
          500: '#e91e63', // Secondary color
          600: '#d81b60',
          700: '#c2185b',
          800: '#ad1457',
          900: '#880e4f',
        },
        // Material Design status colors
        success: colors.green,
        warning: colors.amber,
        error: colors.red,
        info: colors.blue,
      },
      boxShadow: {
        'material-1': '0 2px 1px -1px rgba(0,0,0,0.2), 0 1px 1px 0 rgba(0,0,0,0.14), 0 1px 3px 0 rgba(0,0,0,0.12)',
        'material-2': '0 3px 1px -2px rgba(0,0,0,0.2), 0 2px 2px 0 rgba(0,0,0,0.14), 0 1px 5px 0 rgba(0,0,0,0.12)',
        'material-3': '0 3px 3px -2px rgba(0,0,0,0.2), 0 3px 4px 0 rgba(0,0,0,0.14), 0 1px 8px 0 rgba(0,0,0,0.12)',
        'material-4': '0 2px 4px -1px rgba(0,0,0,0.2), 0 4px 5px 0 rgba(0,0,0,0.14), 0 1px 10px 0 rgba(0,0,0,0.12)',
        // Mobile-optimized shadows
        'mobile-soft': '0 2px 8px rgba(0, 0, 0, 0.1)',
        'mobile-medium': '0 4px 12px rgba(0, 0, 0, 0.15)',
        'mobile-strong': '0 8px 24px rgba(0, 0, 0, 0.2)',
      },
      borderRadius: {
        'material': '4px',
        'mobile': '12px',
        'mobile-lg': '16px',
        'mobile-xl': '20px',
      },
      fontFamily: {
        'material': ['Roboto', 'sans-serif'],
      },
      fontSize: {
        // Mobile-optimized font sizes
        'mobile-xs': ['0.75rem', { lineHeight: '1rem' }],
        'mobile-sm': ['0.875rem', { lineHeight: '1.25rem' }],
        'mobile-base': ['1rem', { lineHeight: '1.5rem' }],
        'mobile-lg': ['1.125rem', { lineHeight: '1.75rem' }],
        'mobile-xl': ['1.25rem', { lineHeight: '1.75rem' }],
        'mobile-2xl': ['1.5rem', { lineHeight: '2rem' }],
        'mobile-3xl': ['1.875rem', { lineHeight: '2.25rem' }],
        'mobile-4xl': ['2.25rem', { lineHeight: '2.5rem' }],
      },
      spacing: {
        // Mobile-optimized spacing
        'mobile-1': '0.25rem',
        'mobile-2': '0.5rem',
        'mobile-3': '0.75rem',
        'mobile-4': '1rem',
        'mobile-5': '1.25rem',
        'mobile-6': '1.5rem',
        'mobile-8': '2rem',
        'mobile-10': '2.5rem',
        'mobile-12': '3rem',
        'mobile-16': '4rem',
        'mobile-20': '5rem',
        'mobile-24': '6rem',
      },
      transitionProperty: {
        'material': 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
        'mobile': 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
      },
      animation: {
        // Mobile-optimized animations
        'fade-in': 'fadeIn 0.3s ease-in-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'slide-down': 'slideDown 0.3s ease-out',
        'scale-in': 'scaleIn 0.2s ease-out',
        'bounce-soft': 'bounceSoft 0.6s ease-in-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideDown: {
          '0%': { transform: 'translateY(-10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        scaleIn: {
          '0%': { transform: 'scale(0.95)', opacity: '0' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
        bounceSoft: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-5px)' },
        },
      },
      screens: {
        // Mobile-first breakpoints
        'xs': '475px',
        'sm': '640px',
        'md': '768px',
        'lg': '1024px',
        'xl': '1280px',
        '2xl': '1536px',
      },
      minHeight: {
        // Mobile touch targets
        'touch': '44px',
        'touch-lg': '48px',
        'touch-xl': '56px',
      },
      minWidth: {
        'touch': '44px',
        'touch-lg': '48px',
        'touch-xl': '56px',
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
    require('@tailwindcss/typography'),
    // Custom plugin for mobile-optimized utilities
    function({ addUtilities, theme }) {
      const newUtilities = {
        '.mobile-safe-area': {
          paddingTop: 'env(safe-area-inset-top)',
          paddingBottom: 'env(safe-area-inset-bottom)',
          paddingLeft: 'env(safe-area-inset-left)',
          paddingRight: 'env(safe-area-inset-right)',
        },
        '.mobile-tap-highlight': {
          '-webkit-tap-highlight-color': 'transparent',
        },
        '.mobile-scroll-smooth': {
          '-webkit-overflow-scrolling': 'touch',
        },
        '.mobile-text-size-adjust': {
          '-webkit-text-size-adjust': '100%',
          '-moz-text-size-adjust': '100%',
          'text-size-adjust': '100%',
        },
        '.mobile-touch-callout': {
          '-webkit-touch-callout': 'none',
        },
        '.mobile-user-select': {
          '-webkit-user-select': 'none',
          '-moz-user-select': 'none',
          '-ms-user-select': 'none',
          'user-select': 'none',
        },
        // Ensure nested scroll containers scroll properly on mobile
        '.mobile-scroll': {
          '-webkit-overflow-scrolling': 'touch',
          'overscroll-behavior': 'contain',
          'touch-action': 'pan-y',
        },
      }
      addUtilities(newUtilities)
    }
  ],
} 