import { invoke } from "@tauri-apps/api/core";

let requestId = 0;

interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: unknown;
}

interface JsonRpcResponse<T = unknown> {
  jsonrpc: "2.0";
  id: number;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export class RpcError extends Error {
  code: number;
  data?: unknown;

  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = "RpcError";
    this.code = code;
    this.data = data;
  }
}

/**
 * Send a JSON-RPC request to the Rust daemon via Tauri IPC bridge.
 * The Tauri command `ipc_request` forwards the request over a Unix socket.
 */
export async function rpcCall<T = unknown>(
  method: string,
  params?: unknown,
): Promise<T> {
  const id = ++requestId;

  const request: JsonRpcRequest = {
    jsonrpc: "2.0",
    id,
    method,
    params,
  };

  const raw = await invoke<string>("ipc_request", {
    body: JSON.stringify(request),
  });

  const response: JsonRpcResponse<T> = JSON.parse(raw);

  if (response.error) {
    throw new RpcError(
      response.error.code,
      response.error.message,
      response.error.data,
    );
  }

  return response.result as T;
}

/**
 * Convenience wrapper for calls that may fail gracefully.
 * Returns { data, error } instead of throwing.
 */
export async function rpcCallSafe<T = unknown>(
  method: string,
  params?: unknown,
): Promise<{ data: T | null; error: RpcError | null }> {
  try {
    const data = await rpcCall<T>(method, params);
    return { data, error: null };
  } catch (err) {
    if (err instanceof RpcError) {
      return { data: null, error: err };
    }
    return {
      data: null,
      error: new RpcError(-1, (err as Error).message),
    };
  }
}
