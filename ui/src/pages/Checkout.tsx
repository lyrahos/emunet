import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Sprout,
  CheckCircle2,
  ArrowLeft,
  Download,
  Shield,
} from "lucide-react";
import Button from "@/components/ui/Button";
import BottomSheet from "@/components/ui/BottomSheet";
import Badge from "@/components/ui/Badge";
import { rpcCall } from "@/lib/rpc-client";
import { useBalance } from "@/hooks/useBalance";
import { useAppStore } from "@/lib/store";
import { formatSeeds, formatBytes } from "@/lib/format";

interface ContentDetail {
  hash: string;
  title: string;
  description: string;
  creatorName: string;
  size: number;
  mimeType: string;
  price: number; // micro-seeds
  owned: boolean;
  tiers?: PricingTier[];
}

interface PricingTier {
  id: string;
  label: string;
  price: number;
  description: string;
}

export default function Checkout() {
  const { contentHash } = useParams<{ contentHash: string }>();
  const navigate = useNavigate();
  const { balance } = useBalance();
  const addToast = useAppStore((s) => s.addToast);

  const [content, setContent] = useState<ContentDetail | null>(null);
  const [sheetOpen, setSheetOpen] = useState(false);
  const [selectedTier, setSelectedTier] = useState<string | null>(null);
  const [purchasing, setPurchasing] = useState(false);

  useEffect(() => {
    if (!contentHash) return;
    rpcCall<ContentDetail>("get_content_detail", { hash: contentHash })
      .then((res) => {
        setContent(res);
        if (res.tiers?.length) {
          setSelectedTier(res.tiers[0].id);
        }
        if (!res.owned && res.price > 0) {
          setSheetOpen(true);
        }
      })
      .catch(() => {});
  }, [contentHash]);

  const effectivePrice =
    content?.tiers?.find((t) => t.id === selectedTier)?.price ?? content?.price ?? 0;

  const canAfford = balance >= effectivePrice;

  const handlePurchase = async () => {
    if (!contentHash || !canAfford) return;
    setPurchasing(true);
    try {
      await rpcCall("purchase_content", {
        hash: contentHash,
        tier_id: selectedTier,
      });
      addToast({
        type: "success",
        title: "Purchase Complete",
        message: `You now own "${content?.title}".`,
      });
      setSheetOpen(false);
      setContent((prev) => (prev ? { ...prev, owned: true } : prev));
    } catch {
      addToast({ type: "error", message: "Purchase failed. Please try again." });
    } finally {
      setPurchasing(false);
    }
  };

  if (!content) {
    return (
      <div className="flex items-center justify-center h-full">
        <motion.div
          animate={{ rotate: 360 }}
          transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
          className="w-6 h-6 border-2 border-[var(--color-accent)] border-t-transparent rounded-full"
        />
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-6">
        {/* Header */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate(-1)}
            className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
          <h1 className="text-xl font-bold text-[var(--color-text)] truncate">
            {content.title}
          </h1>
        </div>

        {/* Content info card */}
        <div className="p-6 rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)]">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-lg font-semibold text-[var(--color-text)]">
                {content.title}
              </h2>
              <p className="text-sm text-[var(--color-text-secondary)] mt-1">
                by {content.creatorName}
              </p>
            </div>
            {content.owned ? (
              <Badge variant="yours">
                <CheckCircle2 className="h-3 w-3 mr-1" />
                Yours
              </Badge>
            ) : content.price === 0 ? (
              <Badge variant="free">Free</Badge>
            ) : (
              <span className="flex items-center gap-1 text-lg font-bold text-[var(--color-accent)]">
                <Sprout className="h-5 w-5" />
                {formatSeeds(content.price)}
              </span>
            )}
          </div>
          {content.description && (
            <p className="text-sm text-[var(--color-text-secondary)] mt-4 leading-relaxed">
              {content.description}
            </p>
          )}
          <div className="flex gap-4 mt-4 text-xs text-[var(--color-text-secondary)]">
            <span>{formatBytes(content.size)}</span>
            <span>{content.mimeType}</span>
          </div>
        </div>

        {/* Actions */}
        {content.owned ? (
          <Button className="w-full" size="lg">
            <Download className="h-4 w-4" />
            Download
          </Button>
        ) : content.price === 0 ? (
          <Button className="w-full" size="lg" onClick={handlePurchase} loading={purchasing}>
            <Download className="h-4 w-4" />
            Get for Free
          </Button>
        ) : (
          <Button className="w-full" size="lg" onClick={() => setSheetOpen(true)}>
            <Sprout className="h-4 w-4" />
            Purchase
          </Button>
        )}

        {/* Privacy note */}
        <div className="flex items-start gap-3 p-4 rounded-xl bg-[var(--color-border)]/10">
          <Shield className="h-5 w-5 text-[var(--color-text-secondary)] flex-shrink-0 mt-0.5" />
          <p className="text-xs text-[var(--color-text-secondary)] leading-relaxed">
            Purchases are private. The creator cannot see your identity unless
            you choose to reveal it.
          </p>
        </div>
      </div>

      {/* Purchase Bottom Sheet */}
      <BottomSheet
        open={sheetOpen}
        onClose={() => setSheetOpen(false)}
        title="Complete Purchase"
      >
        <div className="space-y-4">
          {/* Pricing tiers */}
          {content.tiers && content.tiers.length > 0 ? (
            <div className="space-y-2">
              <p className="text-sm font-medium text-[var(--color-text)]">
                Choose a tier
              </p>
              {content.tiers.map((tier) => (
                <button
                  key={tier.id}
                  onClick={() => setSelectedTier(tier.id)}
                  className={`w-full text-left p-4 rounded-xl border transition-colors ${
                    selectedTier === tier.id
                      ? "border-[var(--color-accent)] bg-[var(--color-accent)]/5"
                      : "border-[var(--color-border)]"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-semibold text-[var(--color-text)]">
                      {tier.label}
                    </span>
                    <span className="flex items-center gap-1 text-sm font-bold text-[var(--color-accent)]">
                      <Sprout className="h-4 w-4" />
                      {formatSeeds(tier.price)}
                    </span>
                  </div>
                  <p className="text-xs text-[var(--color-text-secondary)] mt-1">
                    {tier.description}
                  </p>
                </button>
              ))}
            </div>
          ) : (
            <div className="flex items-center justify-between p-4 rounded-xl border border-[var(--color-border)]">
              <span className="text-sm text-[var(--color-text)]">
                {content.title}
              </span>
              <span className="flex items-center gap-1 text-sm font-bold text-[var(--color-accent)]">
                <Sprout className="h-4 w-4" />
                {formatSeeds(effectivePrice)}
              </span>
            </div>
          )}

          {/* Balance */}
          <div className="flex items-center justify-between py-2 text-sm">
            <span className="text-[var(--color-text-secondary)]">
              Your balance
            </span>
            <span className="font-semibold text-[var(--color-text)]">
              {formatSeeds(balance)}
            </span>
          </div>

          {!canAfford && (
            <p className="text-sm text-red-500 text-center">
              Insufficient Seeds. You need {formatSeeds(effectivePrice - balance)}{" "}
              more.
            </p>
          )}

          <Button
            className="w-full"
            size="lg"
            disabled={!canAfford}
            loading={purchasing}
            onClick={handlePurchase}
          >
            {canAfford
              ? `Pay ${formatSeeds(effectivePrice)}`
              : "Insufficient Seeds"}
          </Button>
        </div>
      </BottomSheet>
    </div>
  );
}
