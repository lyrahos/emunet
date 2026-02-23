import StorefrontGrid from "./StorefrontGrid";
import ForumThread from "./ForumThread";
import NewsFeed from "./NewsFeed";
import GalleryMosaic from "./GalleryMosaic";
import LibraryList from "./LibraryList";

export interface ContentItem {
  hash: string;
  title: string;
  description?: string;
  thumbnail?: string;
  price: number; // micro-seeds, 0 = free
  creatorName: string;
  createdAt: number;
  size: number; // bytes
  mimeType: string;
  owned: boolean;
}

interface LayoutRendererProps {
  template: "storefront" | "forum" | "newsfeed" | "gallery" | "library";
  items: ContentItem[];
  onItemClick: (hash: string) => void;
}

export default function LayoutRenderer({
  template,
  items,
  onItemClick,
}: LayoutRendererProps) {
  switch (template) {
    case "storefront":
      return <StorefrontGrid items={items} onItemClick={onItemClick} />;
    case "forum":
      return <ForumThread items={items} onItemClick={onItemClick} />;
    case "newsfeed":
      return <NewsFeed items={items} onItemClick={onItemClick} />;
    case "gallery":
      return <GalleryMosaic items={items} onItemClick={onItemClick} />;
    case "library":
      return <LibraryList items={items} onItemClick={onItemClick} />;
    default:
      return (
        <div className="p-8 text-center text-[var(--color-text-secondary)]">
          Unknown layout template: {template}
        </div>
      );
  }
}
