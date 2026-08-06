import type { OpenClawConfig } from "../runtime-api.js";
import {
  describeWebhookOutcome,
  executeWebhookAction,
  formatZodError,
  webhookActionSchema,
  type TaskFlowWebhookTarget,
} from "./vendor/webhook-actions.js";

/**
 * Transport-agnostic abstraction over the vendored OpenClaw webhook functions.
 *
 * Upstream's HTTP handler does three things to every request body:
 *   validate (`webhookActionSchema`) → execute (`executeWebhookAction`) →
 *   classify (`describeWebhookOutcome`).
 *
 * That pipeline has nothing to do with HTTP, so we expose it here as a single
 * call. The polling transport (`poller.ts`) feeds it action payloads pulled
 * from a polling endpoint instead of an inbound request, while reusing the
 * exact upstream TaskFlow semantics. Keep this file thin: all behaviour lives
 * in the vendored module so re-syncing upstream needs no changes here.
 */
export type WebhookProcessOutcome = {
  ok: boolean;
  statusCode: number;
  code?: string;
  error?: string;
  result?: unknown;
};

export async function processWebhookAction(params: {
  action: unknown;
  target: TaskFlowWebhookTarget;
  cfg: OpenClawConfig;
}): Promise<WebhookProcessOutcome> {
  const parsed = webhookActionSchema.safeParse(params.action);
  if (!parsed.success) {
    return {
      ok: false,
      statusCode: 400,
      code: "invalid_request",
      error: formatZodError(parsed.error),
    };
  }

  const result = await executeWebhookAction({
    action: parsed.data,
    target: params.target,
    cfg: params.cfg,
  });
  const outcome = describeWebhookOutcome({ action: parsed.data, result });

  return {
    ok: outcome.statusCode < 400,
    statusCode: outcome.statusCode,
    ...(outcome.code ? { code: outcome.code } : {}),
    ...(outcome.error ? { error: outcome.error } : {}),
    result,
  };
}

/** Best-effort label for a polled action, for log lines. */
export function describeActionForLog(action: unknown): string {
  if (action && typeof action === "object" && "action" in action) {
    const value = (action as { action?: unknown }).action;
    if (typeof value === "string") {
      return value;
    }
  }
  return "unknown";
}
