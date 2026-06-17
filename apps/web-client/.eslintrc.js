module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: {
    project: './tsconfig.json',
    tsconfigRootDir: __dirname,
    sourceType: 'module',
  },
  ignorePatterns: ['.eslintrc.js'],
  plugins: ['react', '@typescript-eslint'],
  env: {
    browser: true,
  },
  globals: {
    window: true,
    document: true,
  },
  extends: [
    'airbnb',
    'airbnb-typescript',
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'prettier',
  ],
  rules: {
    'no-bitwise': 'off',
    semi: 'off',
    'arrow-body-style': 'off',
    'react/jsx-no-duplicate-props': 'off',
    'jsx-a11y/anchor-is-valid': 'off',
    'no-underscore-dangle': 'off',
    '@typescript-eslint/naming-convention': 0,
    'react/require-default-props': 0,
    '@typescript-eslint/semi': 0,
    'react/jsx-one-expression-per-line': 0,
    'consistent-return': 0,
    'no-param-reassign': 0,
    'no-named-as-default': 'off',
    'no-nested-ternary': 0,
    'jsx-no-useless-fragment': 'off',
    radix: 0,
    'no-plusplus': 0,
    'prefer-arrow-callback': 0,
    '@typescript-eslint/no-var-requires': 0,
    'import/prefer-default-export': 0,
    'operator-linebreak': 0,
    'object-curly-newline': [
      'error',
      {
        ExportDeclaration: {
          minProperties: 4,
        },
      },
    ],
    'react/jsx-props-no-spreading': 0,
    'jsx-a11y/click-events-have-key-events': 0,
    'jsx-a11y/no-noninteractive-element-interactions': 0,
  },
  settings: {
    node: {
      extensions: ['.ts', '.tsx'],
      'import/resolver': {
        alias: {
          extensions: ['.ts', '.tsx'],
        },
      },
    },
  },
}
