import { create } from "zustand";
import { persist } from "zustand/middleware";

// ---- Types ----

export interface Toast {
  id: string;
  type: "success" | "error" | "info";
  title?: string;
  message: string;
  duration?: number; // ms, 0 = sticky
}

export interface Space {
  groupId: string;
  name: string;
  icon?: string;
  role: "host" | "creator" | "moderator" | "member";
  memberCount: number;
  lastActivity: number; // epoch ms
  unread: boolean;
  pinned: boolean;
  template: "storefront" | "forum" | "newsfeed" | "gallery" | "library";
}

export interface WalletState {
  balance: number; // micro-seeds
  pendingIncoming: number;
  pendingOutgoing: number;
}

export interface AuthState {
  unlocked: boolean;
  pikHash: string | null;
  displayName: string | null;
  setupComplete: boolean;
}

type Theme = "light" | "dark" | "system";

interface AppState {
  // Auth
  auth: AuthState;
  setAuth: (auth: Partial<AuthState>) => void;

  // Spaces
  spaces: Space[];
  activeGroupId: string | null;
  setSpaces: (spaces: Space[]) => void;
  setActiveGroupId: (id: string | null) => void;

  // Wallet
  wallet: WalletState;
  setWallet: (wallet: Partial<WalletState>) => void;

  // Theme
  theme: Theme;
  setTheme: (theme: Theme) => void;

  // Toasts
  toasts: Toast[];
  addToast: (toast: Omit<Toast, "id">) => void;
  removeToast: (id: string) => void;

  // Earning
  earnLevel: number; // 0=low, 1=medium, 2=high, fractional=custom
  storageAllocationGb: number;
  setEarnLevel: (level: number) => void;
  setStorageAllocation: (gb: number) => void;
}

let toastCounter = 0;

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Auth
      auth: {
        unlocked: false,
        pikHash: null,
        displayName: null,
        setupComplete: false,
      },
      setAuth: (partial) =>
        set((s) => ({ auth: { ...s.auth, ...partial } })),

      // Spaces
      spaces: [],
      activeGroupId: null,
      setSpaces: (spaces) => set({ spaces }),
      setActiveGroupId: (id) => set({ activeGroupId: id }),

      // Wallet
      wallet: { balance: 0, pendingIncoming: 0, pendingOutgoing: 0 },
      setWallet: (partial) =>
        set((s) => ({ wallet: { ...s.wallet, ...partial } })),

      // Theme
      theme: "system",
      setTheme: (theme) => set({ theme }),

      // Toasts
      toasts: [],
      addToast: (toast) =>
        set((s) => ({
          toasts: [
            ...s.toasts,
            { ...toast, id: `toast-${++toastCounter}-${Date.now()}` },
          ],
        })),
      removeToast: (id) =>
        set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

      // Earning
      earnLevel: 1,
      storageAllocationGb: 10,
      setEarnLevel: (level) => set({ earnLevel: level }),
      setStorageAllocation: (gb) => set({ storageAllocationGb: gb }),
    }),
    {
      name: "ochra-store",
      partialize: (s) => ({
        theme: s.theme,
        auth: { ...s.auth, unlocked: false },
        earnLevel: s.earnLevel,
        storageAllocationGb: s.storageAllocationGb,
      }),
    },
  ),
);
