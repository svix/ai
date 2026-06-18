import { Svix } from "svix";
import type { PluginLogger } from "../api.js";
import { resolveConfiguredSecretInputString, type OpenClawConfig } from "../runtime-api.js";
import type { ResolvedPoller } from "./config.js";

export type WebhookPoller = {
  start: () => void;
  stop: () => Promise<void>;
};

// Result of handing one polled message to its destination (a TaskFlow action or
// a gateway hook POST). `summary` is a short label for the success log line.
export type DispatchOutcome = {
  ok: boolean;
  status?: number;
  code?: string;
  error?: string;
  summary?: string;
};

export type DispatchFn = (
  action: unknown,
  message: { id: string; eventType: string },
) => Promise<DispatchOutcome>;

function sleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }
    const timer = setTimeout(resolve, ms);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
  });
}

/**
 * One long-lived poller per configured endpoint. Instead of OpenClaw exposing an
 * inbound HTTP route, this reads a Svix Polling Endpoint with the official Svix
 * SDK (`svix.message.poller.poll`) and hands each buffered message's payload to
 * the supplied `dispatch` callback — which either applies it as a TaskFlow
 * action or POSTs it to a gateway hook (`/hooks/wake`, `/hooks/agent`).
 */
export function createWebhookPoller(params: {
  poller: ResolvedPoller;
  cfg: OpenClawConfig;
  logger: PluginLogger;
  dispatch: DispatchFn;
}): WebhookPoller {
  const { poller, cfg, logger, dispatch } = params;
  const controller = new AbortController();
  // The Svix polling cursor is kept in memory only (seeded from optional
  // `startIterator`). It is intentionally NOT persisted: on a gateway
  // restart/reload the poller re-initializes and the cursor resets, per the
  // documented Svix polling-endpoint flow
  // (https://docs.svix.com/receiving/using-app-portal/polling-endpoints).
  let iterator = poller.startIterator;
  let stopped = false;
  let loop: Promise<void> | undefined;

  async function resolveToken(): Promise<string | undefined> {
    if (typeof poller.token === "string") {
      return poller.token;
    }
    const resolved = await resolveConfiguredSecretInputString({
      config: cfg,
      env: process.env,
      value: poller.token,
      path: poller.tokenConfigPath,
    });
    return resolved.value;
  }

  function extractAction(message: Record<string, unknown>): unknown {
    return poller.payloadField ? message[poller.payloadField] : message;
  }

  // Returns whether the Polling Endpoint reports the backlog is drained.
  async function pollOnce(): Promise<boolean> {
    // Resolve per poll so rotating SecretRef tokens are picked up.
    const token = await resolveToken();
    if (!token) {
      logger.warn?.(`[svix-openclaw] ${poller.label} skipped poll: token unresolved`);
      return true;
    }

    const svix = new Svix(token, poller.serverUrl ? { serverUrl: poller.serverUrl } : {});
    const res = await svix.message.poller.poll(poller.appId, poller.sinkId, {
      limit: poller.limit,
      ...(iterator ? { iterator } : {}),
      ...(poller.eventType ? { eventType: poller.eventType } : {}),
      ...(poller.channel ? { channel: poller.channel } : {}),
    });

    for (const message of res.data) {
      if (stopped) {
        break;
      }
      const action = extractAction(message as unknown as Record<string, unknown>);
      const outcome = await dispatch(action, {
        id: String(message.id),
        eventType: String(message.eventType),
      });
      if (outcome.ok) {
        logger.info?.(
          `[svix-openclaw] ${poller.label} dispatched ${outcome.summary ?? "message"} ` +
            `(msg ${message.id} ${message.eventType})` +
            (outcome.status ? ` -> ${outcome.status}` : ""),
        );
      } else {
        logger.warn?.(
          `[svix-openclaw] ${poller.label} rejected msg ${message.id}: ` +
            `${outcome.code ?? "error"} ${outcome.error ?? ""}`.trim(),
        );
      }
    }

    // Advance the cursor so the next poll continues past what we just drained.
    if (res.iterator) {
      iterator = res.iterator;
    }
    return res.done;
  }

  async function run(): Promise<void> {
    while (!stopped) {
      try {
        const done = await pollOnce();
        // When caught up, idle for the configured interval; otherwise keep
        // paging through the backlog immediately.
        if (done && !stopped) {
          await sleep(poller.pollIntervalMs, controller.signal);
        }
      } catch (err) {
        if (stopped) {
          break;
        }
        logger.warn?.(`[svix-openclaw] ${poller.label} error: ${String(err)}`);
        await sleep(poller.pollIntervalMs, controller.signal);
      }
    }
  }

  return {
    start() {
      if (loop) {
        return;
      }
      logger.info?.(
        `[svix-openclaw] polling Svix app=${poller.appId} sink=${poller.sinkId} ` +
          `-> ${poller.kind} (poller ${poller.label})`,
      );
      loop = run();
    },
    async stop() {
      stopped = true;
      controller.abort();
      await loop?.catch(() => {});
    },
  };
}
