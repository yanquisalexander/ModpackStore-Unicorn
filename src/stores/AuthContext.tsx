// AuthContext.tsx
import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from "@tauri-apps/api/core";

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

interface AuthContextType {
  session: any | null;
  loading: boolean;
  error: string | null;
  authStep: AuthStep;
  startDiscordAuth: () => Promise<void>;
  logout: () => Promise<void>;
  isAuthenticated: boolean;
}

interface AuthProviderProps {
  children: ReactNode;
}

// Create context with default values
export const AuthContext = createContext<AuthContextType | null>(null);

// Auth provider component
export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [session, setSession] = useState<UserSession | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [authStep, setAuthStep] = useState<AuthStep>(null);

  // Initialize auth state on load
  useEffect(() => {
    let authStatusUnlisten: UnlistenFn;
    let authErrorUnlisten: UnlistenFn;
    let authStepUnlisten: UnlistenFn;

    const initAuth = async (): Promise<void> => {
      try {
        setLoading(true);

        // Listen for auth status updates from Tauri backend
        authStatusUnlisten = await listen<UserSession | null>('auth-status-changed', (event) => {
          console.log('Auth status changed:', event.payload);
          setSession(event.payload);
          setLoading(false);
          // Reset auth step cuando se completa la autenticación
          setAuthStep(null);
          setError(null);
        });

        // Listen for auth errors from Tauri backend
        authErrorUnlisten = await listen<string>('auth-error', (event) => {
          console.error('Auth error:', event.payload);
          setError(event.payload);
          setLoading(false);
          setAuthStep(null);
        });

        // Listen for auth step updates
        authStepUnlisten = await listen<AuthStep>('auth-step-changed', (event) => {
          console.log('Auth step changed:', event.payload);
          setAuthStep(event.payload);
        });

        // Check current session status from Tauri
        const currentSession = await invoke<UserSession | null>('get_current_session');
        setSession(currentSession);
      } catch (err) {
        console.error('Auth initialization error:', err);
        setError(err instanceof Error ? err.message : 'Failed to initialize authentication');
      } finally {
        setLoading(false);
      }
    };

    initAuth();

    // Clean up listeners on unmount
    return () => {
      if (authStatusUnlisten) authStatusUnlisten();
      if (authErrorUnlisten) authErrorUnlisten();
      if (authStepUnlisten) authStepUnlisten();
    };
  }, []);

  // Start Discord OAuth flow
  const startDiscordAuth = async (): Promise<void> => {
    try {
      setError(null);
      setAuthStep('starting-auth');

      // Invoke the Tauri command to start Discord OAuth flow
      await invoke('start_discord_auth');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start Discord login');
      setAuthStep(null);
      throw err;
    }
  };

  // Logout function
  const logout = async (): Promise<void> => {
    try {
      // Invoke the Tauri command to logout
      await invoke('logout');
      // Reset React state
      setSession(null);
      setError(null);
      setAuthStep(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to logout');
      throw err;
    }
  };

  // Context value
  const value: AuthContextType = {
    session,
    loading,
    error,
    authStep,
    startDiscordAuth,
    logout,
    isAuthenticated: !!session,
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