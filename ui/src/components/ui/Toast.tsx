import { useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, CheckCircle2, AlertCircle, Info } from "lucide-react";
import { useAppStore, type Toast as ToastType } from "@/lib/store";

const iconMap = {
  success: CheckCircle2,
  error: AlertCircle,
  info: Info,
};

const colorMap = {
  success: "text-green-500",
  error: "text-red-500",
  info: "text-blue-500",
};

function ToastItem({ toast }: { toast: ToastType }) {
  const removeToast = useAppStore((s) => s.removeToast);
  const Icon = iconMap[toast.type];

  useEffect(() => {
    if (toast.duration === 0) return;
    const timer = setTimeout(() => {
      removeToast(toast.id);
    }, toast.duration ?? 4000);
    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, removeToast]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 20, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: -10, scale: 0.95 }}
      transition={{ type: "spring", stiffness: 300, damping: 25 }}
      className="flex items-start gap-3 w-80 p-4 rounded-xl bg-[var(--color-surface)] border border-[var(--color-border)] shadow-lg"
    >
      <Icon className={`h-5 w-5 mt-0.5 flex-shrink-0 ${colorMap[toast.type]}`} />
      <div className="flex-1 min-w-0">
        {toast.title && (
          <p className="text-sm font-semibold text-[var(--color-text)]">
            {toast.title}
          </p>
        )}
        <p className="text-sm text-[var(--color-text-secondary)]">
          {toast.message}
        </p>
      </div>
      <button
        onClick={() => removeToast(toast.id)}
        className="flex-shrink-0 rounded-md p-1 text-[var(--color-text-secondary)] hover:text-[var(--color-text)] transition-colors"
      >
        <X className="h-4 w-4" />
      </button>
    </motion.div>
  );
}

export function ToastContainer() {
  const toasts = useAppStore((s) => s.toasts);

  return (
    <div className="fixed bottom-4 right-4 z-[100] flex flex-col-reverse gap-2">
      <AnimatePresence mode="popLayout">
        {toasts.map((toast) => (
          <ToastItem key={toast.id} toast={toast} />
        ))}
      </AnimatePresence>
    </div>
  );
}
