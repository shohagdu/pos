import '@testing-library/jest-dom/vitest'
import '../i18n'

import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

import { setTokens } from '../api/session'

afterEach(() => {
  cleanup()
  // The access token store is module state; never leak a session between tests.
  setTokens(null)
  for (const key of Object.keys(localStorage)) if (key.startsWith('pos.branch.')) localStorage.removeItem(key)
})
