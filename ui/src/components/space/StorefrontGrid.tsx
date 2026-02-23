import { motion } from "framer-motion";
import { Sprout, CheckCircle2 } from "lucide-react";
import Badge from "@/components/ui/Badge";
import { formatSeeds } from "@/lib/format";
import type { ContentItem } from "./LayoutRenderer";

interface StorefrontGridProps {
  items: ContentItem[];
  onItemClick: (hash: string) => void;
}

export default function StorefrontGrid({
  items,
  onItemClick,
}: StorefrontGridProps) {
  return (
    <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      {items.map((item, index) => (
        <motion.button
          key={item.hash}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: index * 0.04 }}
          whileHover={{ y: -3 }}
          whileTap={{ scale: 0.97 }}
          onClick={() => onItemClick(item.hash)}
          className="text-left rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] overflow-hidden hover:shadow-lg transition-shadow"
        >
          {/* Thumbnail */}
          <div className="aspect-[4/3] bg-[var(--color-border)]/30 relative">
            {item.thumbnail ? (
              <img
                src={item.thumbnail}
                alt={item.title}
                className="w-full h-full object-cover"
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center text-[var(--color-text-secondary)]/40">
                <Sprout className="h-10 w-10" />
              </div>
            )}
            {/* Price overlay */}
            <div className="absolute bottom-2 right-2">
              {item.owned ? (
                <Badge variant="yours">
                  <CheckCircle2 className="h-3 w-3 mr-1" />
                  Yours
                </Badge>
              ) : item.price === 0 ? (
                <Badge variant="free">Free</Badge>
              ) : (
                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-[var(--color-surface)]/90 text-[var(--color-text)] backdrop-blur-sm border border-[var(--color-border)]">
                  <Sprout className="h-3 w-3 text-[var(--color-accent)]" />
                  {formatSeeds(item.price)}
                </span>
              )}
            </div>
          </div>

          {/* Content */}
          <div className="p-3">
            <h4 className="text-sm font-semibold text-[var(--color-text)] truncate">
              {item.title}
            </h4>
            <p className="text-xs text-[var(--color-text-secondary)] mt-0.5">
              {item.creatorName}
            </p>
          </div>
        </motion.button>
      ))}
    </div>
  );
}
