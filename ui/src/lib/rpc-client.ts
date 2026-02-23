import { invoke } from "@tauri-apps/api/core";

/** Shape returned by the Tauri `ipc_request` command. */
interface IpcResponse {
  ok: boolean;
  result?: unknown;
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
  // The Tauri command builds the JSON-RPC envelope itself;
  // we only need to pass method + params.
  let response: IpcResponse;
  try {
    response = await invoke<IpcResponse>("ipc_request", {
      request: { method, params: params ?? {} },
    });
  } catch (err) {
    // Tauri rejects with a plain string when the bridge itself fails
    // (e.g. daemon not running, socket not found).
    const msg = typeof err === "string" ? err : (err as Error).message ?? String(err);
    throw new RpcError(-1, msg);
  }

  if (!response.ok || response.error) {
    const err = response.error;
    throw new RpcError(
      err?.code ?? -1,
      err?.message ?? "Unknown daemon error",
      err?.data,
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
