export const LICENSE_ERROR_CODES = ['ACTIVATION_REQUIRED', 'VERIFICATION_REQUIRED', 'INVALID_CARD', 'WRONG_PRODUCT', 'LICENSE_DISABLED', 'LICENSE_EXPIRED', 'DEVICE_MISMATCH', 'RATE_LIMITED', 'INVALID_REQUEST', 'TOKEN_INVALID', 'CLOCK_ROLLBACK', 'CACHE_ERROR', 'NETWORK_UNAVAILABLE', 'SERVICE_UNAVAILABLE', 'LICENSING_UNAVAILABLE'] as const
export type LicenseErrorCode = typeof LICENSE_ERROR_CODES[number]
export type LicensePlan = '7d' | '30d' | '365d' | 'lifetime'
export interface LicenseStatus {
  authorized: boolean
  mode: 'online' | 'offline' | null
  code: LicenseErrorCode | null
  plan: LicensePlan | null
  expiresAt: number | null
  tokenExpiresAt: number | null
  lastVerifiedAt: number | null
  deviceId: string
  maskedCardKey: string | null
  serverUrl: string
  cardPrefix: string
  retryAfterSeconds: number | null
}
export interface LicenseError { code: LicenseErrorCode; retryAfterSeconds: number | null }
