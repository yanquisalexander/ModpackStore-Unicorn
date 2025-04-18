// AuthContext.tsx
import { createContext, useContext, useEffect, useState, ReactNode, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from "@tauri-apps/api/core";
import { load } from '@tauri-apps/plugin-store';

// Define types
interface UserSession {
  id: string;
  name: string;
  email: string;
  avatarUrl?: string;
  roles: string[];
}

// Auth steps para mostrar el estado actual del proceso de autenticación
type AuthStep =
  | null
  | 'starting-auth'
  | 'waiting-callback'
  | 'processing-callback'
  | 'requesting-session';

interface AuthError {
  error_code: string;
  error: string;
}

interface SessionTokens {
  access_token: string;
  refresh_token: string;
}

interface AuthContextType {
  session: UserSession | null;
  loading: boolean;
  error: AuthError | null;
  authStep: AuthStep;
  startDiscordAuth: () => Promise<void>;
  logout: () => Promise<void>;
  isAuthenticated: boolean;
  sessionTokens: SessionTokens | null;
}

interface AuthProviderProps {
  children: ReactNode;
}

// Crear un valor por defecto para el contexto
const defaultContextValue: AuthContextType = {
  session: null,
  loading: true,
  error: null,
  authStep: null,
  startDiscordAuth: async () => { throw new Error('AuthContext not initialized') },
  logout: async () => { throw new Error('AuthContext not initialized') },
  isAuthenticated: false,
  sessionTokens: null
};

// Create context with default values
export const AuthContext = createContext<AuthContextType>(defaultContextValue);

// Auth provider component
export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [session, setSession] = useState<UserSession | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<AuthError | null>(null);
  const [authStep, setAuthStep] = useState<AuthStep>(null);
  const [sessionTokens, setSessionTokens] = useState<SessionTokens | null>(null);

  // Computed value
  const isAuthenticated = Boolean(session);

  // Parse error helper
  const parseError = (err: unknown): AuthError => {
    if (err instanceof Error) {
      return { error_code: 'UNKNOWN_ERROR', error: err.message };
    }

    if (typeof err === 'string') {
      try {
        return JSON.parse(err) as AuthError;
      } catch {
        return { error_code: 'PARSE_ERROR', error: err };
      }
    }

    return { error_code: 'UNKNOWN_ERROR', error: 'Unknown authentication error occurred' };
  };

  // Reset auth state helper
  const resetAuthState = useCallback(() => {
    setAuthStep(null);
    setError(null);
  }, []);

  // Initialize auth state on load
  useEffect(() => {
    const unlistenFunctions: UnlistenFn[] = [];

    const setupListeners = async (): Promise<void> => {
      // Listen for auth status updates from Tauri backend
      const authStatusUnlisten = await listen<UserSession | null>('auth-status-changed', async (event) => {
        try {
          const store = await load('auth_store.json');
          const tokens = await store.get<SessionTokens>('auth_tokens');

          if (tokens) {
            setSessionTokens(tokens);
          }

          setSession(event.payload);
          resetAuthState();
        } catch (err) {
          console.error('Error handling auth status:', err);
          setError(parseError(err));
        } finally {
          setLoading(false);
        }
      });
      unlistenFunctions.push(authStatusUnlisten);

      // Listen for auth errors from Tauri backend
      const authErrorUnlisten = await listen<string>('auth-error', (event) => {
        console.error('Auth error:', event.payload);
        setError(parseError(event.payload));
        setLoading(false);
        setAuthStep(null);
      });
      unlistenFunctions.push(authErrorUnlisten);

      // Listen for auth step updates
      const authStepUnlisten = await listen<AuthStep>('auth-step-changed', (event) => {
        setAuthStep(event.payload);
      });
      unlistenFunctions.push(authStepUnlisten);
    };

    const initAuth = async (): Promise<void> => {
      try {
        setLoading(true);

        // Set up event listeners
        await setupListeners();

        // Initialize auth on rust side
        await invoke('init_session');
      } catch (err) {
        console.error('Auth initialization error:', err);
        setError(parseError(err));
      } finally {
        setLoading(false);
      }
    };

    // Start the auth initialization
    initAuth();

    // Clean up listeners on unmount
    return () => {
      unlistenFunctions.forEach(unlisten => unlisten());
    };
  }, [resetAuthState]);

  // Start Discord OAuth flow
  const startDiscordAuth = useCallback(async (): Promise<void> => {
    try {
      setError(null);
      setAuthStep('starting-auth');

      // Invoke the Tauri command to start Discord OAuth flow
      await invoke('start_discord_auth');
    } catch (err) {
      const parsedError = parseError(err);
      setError({ ...parsedError, error_code: 'DISCORD_AUTH_ERROR' });
      setAuthStep(null);
      throw err;
    }
  }, []);

  // Logout function
  const logout = useCallback(async (): Promise<void> => {
    try {
      // Invoke the Tauri command to logout
      await invoke('logout');

      // Reset React state
      setSession(null);
      setSessionTokens(null);
      resetAuthState();
    } catch (err) {
      const parsedError = parseError(err);
      setError({ ...parsedError, error_code: 'LOGOUT_ERROR' });
      throw err;
    }
  }, [resetAuthState]);

  // Context value
  const value: AuthContextType = {
    session,
    loading,
    error,
    authStep,
    startDiscordAuth,
    logout,
    isAuthenticated,
    sessionTokens,
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
};

// Custom hook for using auth context
export const useAuthentication = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuthentication must be used within an AuthProvider');
  }
  return context;
};