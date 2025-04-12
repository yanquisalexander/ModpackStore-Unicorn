// AuthContext.tsx
import { createContext, useContext, useEffect, useState, ReactNode } from 'react';

// Define types
interface UserSession {
  id: string;
  name: string;
  email: string;
  avatarUrl?: string;
  roles: string[];
}

interface AuthContextType {
  session: UserSession | null;
  loading: boolean;
  error: string | null;
  startOAuthFlow: (provider: OAuthProvider) => Promise<void>;
  logout: () => void;
  isAuthenticated: boolean;
  handleOAuthCallback: (code: string, state: string) => Promise<void>;
}

declare global {
  interface Window {
    refreshTokenInterval: number | undefined;
  }
}


type OAuthProvider = 'google' | 'github' | 'microsoft';

interface TokenResponse {
  token: string;
  refreshToken: string;
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

  // Initialize auth state on load
  useEffect(() => {
    const initAuth = async (): Promise<void> => {
      try {
        setLoading(true);
        // Check for OAuth callback
        const urlParams = new URLSearchParams(window.location.search);
        const code = urlParams.get('code');
        const state = urlParams.get('state');

        // If we have OAuth code/state in URL, process it
        if (code && state) {
          await handleOAuthCallback(code, state);
          // Clean URL after processing OAuth callback
          window.history.replaceState({}, document.title, window.location.pathname);
          return;
        }

        // Otherwise try to get stored tokens
        const token = localStorage.getItem('auth_token');
        const refreshToken = localStorage.getItem('refresh_token');

        if (token && refreshToken) {
          // Fetch user session with the token
          await fetchUserSession(token);

          // Set up token refresh interval
          setupRefreshInterval(refreshToken);
        } else {
          // No stored tokens, user is not authenticated
          setSession(null);
        }
      } catch (err) {
        console.error('Auth initialization error:', err);
        setError(err instanceof Error ? err.message : 'Failed to initialize authentication');
        // Clear invalid tokens
        localStorage.removeItem('auth_token');
        localStorage.removeItem('refresh_token');
      } finally {
        setLoading(false);
      }
    };

    initAuth();

    // Clean up on unmount
    return () => {
      if (window.refreshTokenInterval) {
        clearInterval(window.refreshTokenInterval);
      }
    };
  }, []);

  // Fetch user session with token
  const fetchUserSession = async (token: string): Promise<void> => {
    try {
      // Call your API to get user data with the token
      const response = await fetch('YOUR_API_ENDPOINT/me', {
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });

      if (!response.ok) {
        throw new Error('Failed to fetch user session');
      }

      const userData = await response.json() as UserSession;
      setSession(userData);
    } catch (err) {
      throw err;
    }
  };

  // Set up refresh token interval
  const setupRefreshInterval = (refreshToken: string): void => {
    // Clear any existing interval
    if (window.refreshTokenInterval) {
      clearInterval(window.refreshTokenInterval);
    }

    // Add refreshTokenInterval to Window interface

    // Refresh token every 14 minutes (assuming a 15-minute expiry)
    window.refreshTokenInterval = window.setInterval(async () => {
      try {
        const response = await fetch('YOUR_API_ENDPOINT/refresh', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ refreshToken })
        });

        if (!response.ok) {
          throw new Error('Failed to refresh token');
        }

        const { token: newToken, refreshToken: newRefreshToken } = await response.json() as TokenResponse;

        // Update tokens in localStorage
        localStorage.setItem('auth_token', newToken);
        localStorage.setItem('refresh_token', newRefreshToken);

        // Fetch user session with new token
        await fetchUserSession(newToken);
      } catch (err) {
        console.error('Token refresh error:', err);
        // Handle refresh failure - log user out
        logout();
      }
    }, 14 * 60 * 1000); // 14 minutes in milliseconds
  };

  // Start OAuth flow
  const startOAuthFlow = async (provider: OAuthProvider): Promise<void> => {
    try {
      setLoading(true);
      // Generate state for CSRF protection
      const state = crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);
      localStorage.setItem('oauth_state', state);

      // Get OAuth URL from your API
      const response = await fetch(`YOUR_API_ENDPOINT/oauth/url/${provider}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          state,
          redirectUri: window.location.origin,
        }),
      });

      if (!response.ok) {
        throw new Error(`Failed to get OAuth URL for ${provider}`);
      }

      const { url } = await response.json();

      // Redirect to the OAuth provider
      window.location.href = url;
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to start ${provider} login`);
      setLoading(false);
      throw err;
    }
  };

  // Handle OAuth callback
  const handleOAuthCallback = async (code: string, state: string): Promise<void> => {
    try {
      setLoading(true);

      // Verify state to prevent CSRF attacks
      const savedState = localStorage.getItem('oauth_state');
      if (!savedState || savedState !== state) {
        throw new Error('Invalid OAuth state');
      }

      // Clear the saved state
      localStorage.removeItem('oauth_state');

      // Exchange code for tokens
      const response = await fetch('YOUR_API_ENDPOINT/oauth/callback', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          code,
          state,
          redirectUri: window.location.origin,
        })
      });

      if (!response.ok) {
        throw new Error('Failed to complete OAuth authentication');
      }

      const { token, refreshToken } = await response.json() as TokenResponse;

      // Store tokens
      localStorage.setItem('auth_token', token);
      localStorage.setItem('refresh_token', refreshToken);

      // Fetch user session
      await fetchUserSession(token);

      // Set up token refresh
      setupRefreshInterval(refreshToken);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'OAuth authentication failed');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Logout function
  const logout = (): void => {
    // Clear tokens from localStorage
    localStorage.removeItem('auth_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('oauth_state');

    // Clear interval
    if (window.refreshTokenInterval) {
      clearInterval(window.refreshTokenInterval);
    }

    // Reset state
    setSession(null);
    setError(null);
  };

  // Context value
  const value: AuthContextType = {
    session,
    loading,
    error,
    startOAuthFlow,
    logout,
    isAuthenticated: !!session,
    handleOAuthCallback,
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
