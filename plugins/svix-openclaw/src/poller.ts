import { AutoConfigConsumer } from "svix";
import type { PluginLogger } from "../api.js";
import { resolveConfiguredSecretInputString, type OpenClawConfig } from "../runtime-api.js";
import type { ResolvedPoller } from "./config.js";

// `SinkInCommon` and the poll-options type are not re-exported from the `svix`
// package root, so derive them from the `AutoConfigConsumer` signature.
type SinkIn = ConstructorParameters<typeof AutoConfigConsumer>[1];
type ReceiveOptions = NonNullable<Parameters<AutoConfigConsumer["receive"]>[1]>;

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

// Sink-provisioning config passed to `subscribe()`. Only includes fields that
// have real values so the SDK omits the rest (an explicit `null` would be
// emitted and rejected by the server).
function buildSinkIn(poller: ResolvedPoller): SinkIn {
  const sink: SinkIn = {};
  if (poller.filterTypes) {
    sink.filterTypes = poller.filterTypes;
  }
  if (poller.channels) {
    sink.channels = poller.channels;
  }
  return sink;
}

/**
 * One long-lived poller per configured endpoint. Instead of OpenClaw exposing an
 * inbound HTTP route, this drains a Svix polling sink with the official SDK's
 * `AutoConfigConsumer` and hands each buffered message's payload to the supplied
 * `dispatch` callback — which either applies it as a TaskFlow action or POSTs it
 * to a gateway hook (`/hooks/wake`, `/hooks/agent`).
 *
 * The consumer is offset/lease based: `receive()` leases a batch, and once it
 * is dispatched `commit()` acks every message up to the highest offset and
 * releases the lease so the next `receive()` returns the following batch. The
 * cursor lives server-side under a deterministic `consumerId`, so a restart
 * resumes where it left off.
 */
export function createWebhookPoller(params: {
  poller: ResolvedPoller;
  cfg: OpenClawConfig;
  logger: PluginLogger;
  dispatch: DispatchFn;
}): WebhookPoller {
  const { poller, cfg, logger, dispatch } = params;
  const controller = new AbortController();
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

  // Build a consumer for the current (possibly rotated) token. Cheap: it just
  // decodes the AutoConfig token and builds an HTTP request context.
  async function makeConsumer(): Promise<AutoConfigConsumer | undefined> {
    const token = await resolveToken();
    if (!token) {
      logger.warn?.(`[svix-openclaw] ${poller.label} skipped: token unresolved`);
      return undefined;
    }
    return new AutoConfigConsumer(token, buildSinkIn(poller));
  }

  function extractAction(message: Record<string, unknown>): unknown {
    return poller.payloadField ? message[poller.payloadField] : message;
  }

  // Provision the polling sink so the plugin self-configures instead of relying
  // on a sink created by hand in the portal. Best-effort: a failure (e.g. the
  // sink already exists and is managed elsewhere) is logged and polling
  // continues.
  async function provision(): Promise<void> {
    if (!poller.subscribe) {
      return;
    }
    const consumer = await makeConsumer();
    if (!consumer) {
      return;
    }
    try {
      await consumer.subscribe();
      logger.info?.(`[svix-openclaw] ${poller.label} provisioned polling sink`);
    } catch (err) {
      logger.warn?.(
        `[svix-openclaw] ${poller.label} subscribe failed (continuing): ${String(err)}`,
      );
    }
  }

  // Returns whether the sink reports the backlog is drained.
  async function pollOnce(): Promise<boolean> {
    const consumer = await makeConsumer();
    if (!consumer) {
      return true;
    }

    const options: ReceiveOptions = {
      limit: poller.limit,
      ...(poller.leaseDurationMs ? { leaseDurationMs: poller.leaseDurationMs } : {}),
      ...(poller.startingPosition
        ? { startingPosition: poller.startingPosition as ReceiveOptions["startingPosition"] }
        : {}),
    };
    const res = await consumer.receive(poller.consumerId, options);

    // Track the highest offset we actually processed so the commit acks exactly
    // the messages we dispatched and releases the lease.
    let maxOffset: number | undefined;
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
      if (maxOffset === undefined || message.offset > maxOffset) {
        maxOffset = message.offset;
      }
    }

    // Ack everything we dispatched (advances the cursor past this batch) and
    // release the lease so the next receive() returns the following batch.
    if (maxOffset !== undefined) {
      await consumer.commit(poller.consumerId, maxOffset);
    }
    return res.done;
  }

  async function run(): Promise<void> {
    await provision();
    while (!stopped) {
      try {
        const done = await pollOnce();
        // When caught up, idle for the configured interval; otherwise keep
        // draining the backlog immediately.
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
        `[svix-openclaw] polling Svix consumer=${poller.consumerId} ` +
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
