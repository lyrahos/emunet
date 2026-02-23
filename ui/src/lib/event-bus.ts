import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { rpcCall } from "./rpc-client";

type EventHandler = (payload: unknown) => void;

class EventBus {
  private subscriptions = new Map<string, Set<EventHandler>>();
  private tauriUnlisten: UnlistenFn | null = null;
  private initialized = false;

  /**
   * Initialize the event bus by listening to the Tauri event bridge.
   * The daemon sends events via `daemon_event` Tauri event.
   */
  async init(): Promise<void> {
    if (this.initialized) return;
    this.initialized = true;

    this.tauriUnlisten = await listen<{ topic: string; payload: unknown }>(
      "daemon_event",
      (event) => {
        const { topic, payload } = event.payload;
        const handlers = this.subscriptions.get(topic);
        if (handlers) {
          handlers.forEach((handler) => {
            try {
              handler(payload);
            } catch (err) {
              console.error(`[EventBus] handler error for "${topic}":`, err);
            }
          });
        }
      },
    );
  }

  /**
   * Subscribe to a daemon event topic.
   * Also sends subscribe_events RPC to the daemon so it starts forwarding.
   */
  async subscribe(topic: string, handler: EventHandler): Promise<() => void> {
    if (!this.subscriptions.has(topic)) {
      this.subscriptions.set(topic, new Set());
      // Notify daemon we want this topic
      try {
        await rpcCall("subscribe_events", { topics: [topic] });
      } catch {
        // Daemon may not be ready yet; we'll still register locally
      }
    }

    this.subscriptions.get(topic)!.add(handler);

    // Return unsubscribe function
    return () => {
      const handlers = this.subscriptions.get(topic);
      if (handlers) {
        handlers.delete(handler);
        if (handlers.size === 0) {
          this.subscriptions.delete(topic);
          rpcCall("unsubscribe_events", { topics: [topic] }).catch(() => {});
        }
      }
    };
  }

  /**
   * Tear down the event bus.
   */
  destroy(): void {
    if (this.tauriUnlisten) {
      this.tauriUnlisten();
      this.tauriUnlisten = null;
    }
    this.subscriptions.clear();
    this.initialized = false;
  }
}

export const eventBus = new EventBus();
