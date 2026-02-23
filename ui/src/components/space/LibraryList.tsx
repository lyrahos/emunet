import { useState } from "react";
import { motion } from "framer-motion";
import {
  ArrowUpDown,
  FileText,
  Sprout,
  CheckCircle2,
  ChevronDown,
} from "lucide-react";
import Badge from "@/components/ui/Badge";
import { formatSeeds, formatBytes, formatRelativeTime } from "@/lib/format";
import type { ContentItem } from "./LayoutRenderer";

type SortField = "title" | "createdAt" | "size" | "price";
type SortDir = "asc" | "desc";

interface LibraryListProps {
  items: ContentItem[];
  onItemClick: (hash: string) => void;
}

export default function LibraryList({ items, onItemClick }: LibraryListProps) {
  const [sortField, setSortField] = useState<SortField>("createdAt");
  const [sortDir, setSortDir] = useState<SortDir>("desc");

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortField(field);
      setSortDir("desc");
    }
  };

  const sorted = [...items].sort((a, b) => {
    const mul = sortDir === "asc" ? 1 : -1;
    switch (sortField) {
      case "title":
        return mul * a.title.localeCompare(b.title);
      case "createdAt":
        return mul * (a.createdAt - b.createdAt);
      case "size":
        return mul * (a.size - b.size);
      case "price":
        return mul * (a.price - b.price);
      default:
        return 0;
    }
  });

  const SortButton = ({
    field,
    label,
  }: {
    field: SortField;
    label: string;
  }) => (
    <button
      onClick={() => toggleSort(field)}
      className="flex items-center gap-1 text-xs font-medium text-[var(--color-text-secondary)] hover:text-[var(--color-text)] transition-colors"
    >
      {label}
      {sortField === field && (
        <ChevronDown
          className={`h-3 w-3 transition-transform ${sortDir === "asc" ? "rotate-180" : ""}`}
        />
      )}
    </button>
  );

  return (
    <div>
      {/* Sort header */}
      <div className="flex items-center gap-6 px-4 py-2 border-b border-[var(--color-border)] mb-2">
        <ArrowUpDown className="h-3.5 w-3.5 text-[var(--color-text-secondary)]" />
        <div className="flex-1">
          <SortButton field="title" label="Name" />
        </div>
        <div className="w-24">
          <SortButton field="createdAt" label="Date" />
        </div>
        <div className="w-20">
          <SortButton field="size" label="Size" />
        </div>
        <div className="w-24">
          <SortButton field="price" label="Price" />
        </div>
      </div>

      {/* List items */}
      <div className="space-y-1">
        {sorted.map((item, index) => (
          <motion.button
            key={item.hash}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: index * 0.02 }}
            whileHover={{ backgroundColor: "var(--color-border)" }}
            onClick={() => onItemClick(item.hash)}
            className="w-full text-left flex items-center gap-6 px-4 py-3 rounded-lg transition-colors"
          >
            <FileText className="h-4 w-4 text-[var(--color-text-secondary)] flex-shrink-0" />
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-[var(--color-text)] truncate">
                {item.title}
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                {item.creatorName}
              </p>
            </div>
            <span className="w-24 text-xs text-[var(--color-text-secondary)]">
              {formatRelativeTime(item.createdAt)}
            </span>
            <span className="w-20 text-xs text-[var(--color-text-secondary)]">
              {formatBytes(item.size)}
            </span>
            <div className="w-24">
              {item.owned ? (
                <Badge variant="yours" size="sm">
                  <CheckCircle2 className="h-3 w-3 mr-0.5" />
                  Yours
                </Badge>
              ) : item.price === 0 ? (
                <Badge variant="free" size="sm">
                  Free
                </Badge>
              ) : (
                <span className="flex items-center gap-1 text-xs font-semibold text-[var(--color-accent)]">
                  <Sprout className="h-3 w-3" />
                  {formatSeeds(item.price)}
                </span>
              )}
            </div>
          </motion.button>
        ))}
      </div>
    </div>
  );
}
