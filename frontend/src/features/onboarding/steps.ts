import type { OnboardingStep } from '../../api/tenancy'

/** Wizard screens in order (Requirement Analysis §7.2 steps 1–5). Payment
 * methods (step 6) arrive with the money module; products and staff are
 * setup-checklist items. */
export const WIZARD_STEPS = ['account', 'business', 'regional', 'branch', 'invoice'] as const
export type WizardStep = (typeof WIZARD_STEPS)[number]

export function nextStep(step: WizardStep): string {
  const i = WIZARD_STEPS.indexOf(step)
  const next = WIZARD_STEPS[i + 1]
  return next ? `/onboarding/${next}` : '/'
}

export function isWizardStep(step: string | undefined): step is Exclude<WizardStep, 'account' | 'business'> {
  return step === 'regional' || step === 'branch' || step === 'invoice'
}

export const CHECKLIST: { key: OnboardingStep; to?: string; skippable?: boolean; comingSoon?: boolean }[] = [
  { key: 'business' },
  { key: 'regional', to: '/onboarding/regional' },
  { key: 'branch', to: '/onboarding/branch' },
  { key: 'invoice', to: '/onboarding/invoice' },
  { key: 'products', skippable: true, comingSoon: true },
  { key: 'staff', to: '/users/invite', skippable: true },
]
