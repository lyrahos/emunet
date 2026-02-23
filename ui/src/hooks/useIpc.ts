import { useState, useCallback, useRef } from "react";
import { rpcCall, RpcError } from "@/lib/rpc-client";

interface UseIpcResult<T> {
  call: (method: string, params?: unknown) => Promise<T | null>;
  data: T | null;
  loading: boolean;
  error: RpcError | null;
  reset: () => void;
}

/**
 * Hook for making JSON-RPC calls to the daemon via Tauri IPC.
 * Provides loading/error states and automatic cancellation on unmount.
 */
export function useIpc<T = unknown>(): UseIpcResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<RpcError | null>(null);
  const mountedRef = useRef(true);

  // Track mounted state for cleanup
  useState(() => {
    return () => {
      mountedRef.current = false;
    };
  });

  const call = useCallback(async (method: string, params?: unknown): Promise<T | null> => {
    setLoading(true);
    setError(null);

    try {
      const result = await rpcCall<T>(method, params);
      if (mountedRef.current) {
        setData(result);
        setLoading(false);
      }
      return result;
    } catch (err) {
      if (mountedRef.current) {
        const rpcError =
          err instanceof RpcError
            ? err
            : new RpcError(-1, (err as Error).message);
        setError(rpcError);
        setLoading(false);
      }
      return null;
    }
  }, []);

  const reset = useCallback(() => {
    setData(null);
    setError(null);
    setLoading(false);
  }, []);

  return { call, data, loading, error, reset };
}
