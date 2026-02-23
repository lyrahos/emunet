import { useEffect, useCallback } from "react";
import { useAppStore } from "@/lib/store";
import { rpcCall } from "@/lib/rpc-client";
import { useEvents } from "./useEvents";

interface WalletBalanceResponse {
  balance: number;
  pending_incoming: number;
  pending_outgoing: number;
}

/**
 * Hook that fetches and keeps the wallet balance in sync.
 * Listens for balance_changed events from the daemon.
 */
export function useBalance() {
  const wallet = useAppStore((s) => s.wallet);
  const setWallet = useAppStore((s) => s.setWallet);
  const unlocked = useAppStore((s) => s.auth.unlocked);

  const fetchBalance = useCallback(async () => {
    try {
      const res = await rpcCall<WalletBalanceResponse>("get_wallet_balance");
      setWallet({
        balance: res.balance,
        pendingIncoming: res.pending_incoming,
        pendingOutgoing: res.pending_outgoing,
      });
    } catch {
      // Wallet may not be available yet
    }
  }, [setWallet]);

  // Initial fetch when unlocked
  useEffect(() => {
    if (unlocked) {
      fetchBalance();
    }
  }, [unlocked, fetchBalance]);

  // Listen for balance changes
  useEvents(
    "balance_changed",
    () => {
      fetchBalance();
    },
    unlocked,
  );

  return {
    balance: wallet.balance,
    pendingIncoming: wallet.pendingIncoming,
    pendingOutgoing: wallet.pendingOutgoing,
    refetch: fetchBalance,
  };
}
