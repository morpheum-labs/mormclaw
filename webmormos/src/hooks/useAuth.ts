import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  type ReactNode,
} from 'react';
import React from 'react';
import {
  getToken as readToken,
  setToken as writeToken,
  clearToken as removeToken,
  isAuthenticated as checkAuth,
  TOKEN_STORAGE_KEY,
} from '../lib/auth';
import {
  pair as apiPair,
  getPublicHealth,
  loginWithTotp as apiLoginWithTotp,
  type AuthMode,
} from '../lib/api';

// ---------------------------------------------------------------------------
// Context shape
// ---------------------------------------------------------------------------

export interface AuthState {
  /** The current bearer token, or null if not authenticated. */
  token: string | null;
  /** Whether the user is currently authenticated. */
  isAuthenticated: boolean;
  /** True while the initial auth check is in progress. */
  loading: boolean;
  /** Auth mode from health: pairing | totp | totp_enrollment (when require_pairing). */
  authMode: AuthMode | undefined;
  /** Pair with the agent using a pairing code. Stores the token on success. */
  pair: (code: string) => Promise<void>;
  /** Login with TOTP code. Stores the token on success. */
  loginWithTotp: (code: string) => Promise<void>;
  /** Clear the stored token and sign out. */
  logout: () => void;
}

const AuthContext = createContext<AuthState | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export interface AuthProviderProps {
  children: ReactNode;
}

function fetchHealthAndAuthMode(
  setAuthMode: (m: AuthMode | undefined) => void,
  setAuthenticated: (v: boolean) => void,
): Promise<void> {
  return getPublicHealth().then((health) => {
    if (!health.require_pairing) {
      setAuthenticated(true);
      setAuthMode(undefined);
    } else {
      setAuthMode(health.auth_mode ?? 'pairing');
    }
  });
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [token, setTokenState] = useState<string | null>(readToken);
  const [authenticated, setAuthenticated] = useState<boolean>(checkAuth);
  const [authMode, setAuthMode] = useState<AuthMode | undefined>(undefined);
  const [loading, setLoading] = useState<boolean>(!checkAuth());

  // On mount: check if server requires pairing at all
  useEffect(() => {
    if (checkAuth()) return; // already have a token, no need to check
    let cancelled = false;
    getPublicHealth()
      .then((health) => {
        if (cancelled) return;
        if (!health.require_pairing) {
          setAuthenticated(true);
        } else {
          setAuthMode(health.auth_mode ?? 'pairing');
        }
      })
      .catch(() => {
        // health endpoint unreachable — fall back to showing pairing dialog
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // When authenticated becomes false (e.g. after logout), re-fetch health for fresh auth_mode
  const prevAuthenticatedRef = React.useRef(authenticated);
  useEffect(() => {
    if (prevAuthenticatedRef.current && !authenticated) {
      setLoading(true);
      fetchHealthAndAuthMode(setAuthMode, setAuthenticated)
        .catch(() => {})
        .finally(() => setLoading(false));
    }
    prevAuthenticatedRef.current = authenticated;
  }, [authenticated]);

  // Keep state in sync if token storage is changed from another browser context.
  useEffect(() => {
    const handler = (e: StorageEvent) => {
      if (e.key === TOKEN_STORAGE_KEY) {
        const t = readToken();
        setTokenState(t);
        setAuthenticated(t !== null && t.length > 0);
      }
    };
    window.addEventListener('storage', handler);
    return () => window.removeEventListener('storage', handler);
  }, []);

  const pair = useCallback(async (code: string): Promise<void> => {
    const { token: newToken } = await apiPair(code);
    writeToken(newToken);
    setTokenState(newToken);
    setAuthenticated(true);
  }, []);

  const loginWithTotp = useCallback(async (code: string): Promise<void> => {
    const { token: newToken } = await apiLoginWithTotp(code);
    writeToken(newToken);
    setTokenState(newToken);
    setAuthenticated(true);
  }, []);

  const logout = useCallback((): void => {
    removeToken();
    setTokenState(null);
    setAuthenticated(false);
  }, []);

  const value: AuthState = {
    token,
    isAuthenticated: authenticated,
    loading,
    authMode,
    pair,
    loginWithTotp,
    logout,
  };

  return React.createElement(AuthContext.Provider, { value }, children);
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Access the authentication state from any component inside `<AuthProvider>`.
 * Throws if used outside the provider.
 */
export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error('useAuth must be used within an <AuthProvider>');
  }
  return ctx;
}
