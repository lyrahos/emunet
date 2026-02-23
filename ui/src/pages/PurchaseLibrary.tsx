import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  ShoppingBag,
  Download,
  CheckCircle2,
  AlertCircle,
} from "lucide-react";
import Badge from "@/components/ui/Badge";
import { rpcCall } from "@/lib/rpc-client";
import { formatSeeds, formatRelativeTime } from "@/lib/format";

interface Purchase {
  hash: string;
  title: string;
  creatorName: string;
  spaceName: string;
  price: number; // micro-seeds
  purchasedAt: number;
  size: number;
  status: "active" | "expired" | "downloading";
}

export default function PurchaseLibrary() {
  const navigate = useNavigate();
  const [purchases, setPurchases] = useState<Purchase[]>([]);

  useEffect(() => {
    rpcCall<Purchase[]>("get_purchases")
      .then(setPurchases)
      .catch(() => {});
  }, []);

  const statusConfig = {
    active: {
      badge: "yours" as const,
      icon: CheckCircle2,
      label: "Active",
    },
    expired: {
      badge: "expired" as const,
      icon: AlertCircle,
      label: "Expired",
    },
    downloading: {
      badge: "yours" as const,
      icon: Download,
      label: "Downloading",
    },
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-3xl mx-auto px-6 py-8 space-y-6">
        <h1 className="text-xl font-bold text-[var(--color-text)]">
          Purchases
        </h1>

        {purchases.length > 0 ? (
          <div className="space-y-2">
            {purchases.map((purchase, i) => {
              const config = statusConfig[purchase.status];
              return (
                <motion.button
                  key={purchase.hash}
                  initial={{ opacity: 0, y: 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.03 }}
                  whileHover={{ x: 2 }}
                  onClick={() => navigate(`/checkout/${purchase.hash}`)}
                  className="w-full text-left flex items-center gap-4 p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:shadow-sm transition-shadow"
                >
                  <div className="w-12 h-12 rounded-xl bg-[var(--color-accent)]/10 flex items-center justify-center flex-shrink-0">
                    <ShoppingBag className="h-6 w-6 text-[var(--color-accent)]" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <h3 className="text-sm font-semibold text-[var(--color-text)] truncate">
                        {purchase.title}
                      </h3>
                      <Badge variant={config.badge} size="sm">
                        <config.icon className="h-3 w-3 mr-0.5" />
                        {config.label}
                      </Badge>
                    </div>
                    <p className="text-xs text-[var(--color-text-secondary)] mt-0.5">
                      {purchase.creatorName} in {purchase.spaceName}
                    </p>
                  </div>
                  <div className="text-right flex-shrink-0">
                    <p className="text-sm font-semibold text-[var(--color-text)]">
                      {formatSeeds(purchase.price)}
                    </p>
                    <p className="text-xs text-[var(--color-text-secondary)]">
                      {formatRelativeTime(purchase.purchasedAt)}
                    </p>
                  </div>
                </motion.button>
              );
            })}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <div className="w-16 h-16 rounded-2xl bg-[var(--color-accent)]/10 flex items-center justify-center mb-4">
              <ShoppingBag className="h-8 w-8 text-[var(--color-accent)]" />
            </div>
            <h2 className="text-lg font-semibold text-[var(--color-text)]">
              No Purchases Yet
            </h2>
            <p className="text-sm text-[var(--color-text-secondary)] mt-1 max-w-xs">
              Content you purchase from Spaces will appear here.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
