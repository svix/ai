import { z } from "zod";

// Secret-input shape mirrors the OpenClaw webhooks extension: either an inline
// string or a `{ source, provider, id }` secret reference resolved at runtime.
// The vendored action module imports `WebhookSecretInput` from here.
const secretRefSchema = z
  .object({
    source: z.enum(["env", "file", "exec"]),
    provider: z.string().trim().min(1),
    id: z.string().trim().min(1),
  })
  .strict();

const secretInputSchema = z.union([z.string().trim().min(1), secretRefSchema]);

export type WebhookSecretInput = z.infer<typeof secretInputSchema>;

// Sink-provisioning fields applied once via `AutoConfigConsumer.subscribe()`.
// The AutoConfig token already names the application + polling sink, so these
// only describe what the sink should buffer. Spread into each endpoint schema.
const sinkSubscribeFields = {
  // Provision (create/update) the polling sink on startup. Disable when the
  // sink is managed elsewhere and you only want to consume it.
  subscribe: z.boolean().optional().default(true),
  // Sink-level filters applied when the sink is provisioned: narrow what this
  // destination buffers by event type and/or channel.
  filterTypes: z.array(z.string().trim().min(1)).min(1).optional(),
  channels: z.array(z.string().trim().min(1)).min(1).optional(),
};

// Poll tuning shared by every poller (the TaskFlow routes and the per-hook
// endpoints). Spread into each endpoint schema so each poller is tuned
// independently.
const pollTuningFields = {
  // Deterministic consumer id. The Svix server tracks the offset cursor under
  // it, so a gateway restart/reload resumes where it left off. Defaults to
  // `svix-openclaw.<routeId>`.
  consumerId: z.string().trim().min(1).optional(),
  // Where a brand-new consumer starts reading. Only honored on a consumer's
  // first poll; ignored once the server is tracking its offset (default latest).
  startingPosition: z.enum(["earliest", "latest"]).optional(),
  // Lease duration (ms) for a polled batch. Left to the server default if unset.
  leaseDurationMs: z.number().int().positive().optional(),
  // Idle wait (ms) after the endpoint reports it is caught up (`done: true`).
  pollIntervalMs: z.number().int().positive().optional().default(5_000),
  // Page size requested per poll.
  limit: z.number().int().positive().max(500).optional().default(50),
  // Field on each polled message that holds the action/body object. Defaults to
  // the Svix message `payload`. Empty string means the whole message object.
  payloadField: z.string().optional().default("payload"),
};

// A polling sink whose buffered messages are forwarded to one of the OpenClaw
// gateway hooks (`/hooks/wake` or `/hooks/agent`). Each message's payload is
// used verbatim as the hook request body.
const hookEndpointSchema = z
  .object({
    // Svix AutoConfig token (`auto_v1_…`). It embeds the application id, polling
    // sink id, server URL, and API token, so no separate URL is needed. Inline
    // string or a `{ source, provider, id }` secret ref.
    token: secretInputSchema,
    ...sinkSubscribeFields,
    ...pollTuningFields,
  })
  .strict();

// A TaskFlow route: a polling sink whose messages are applied as TaskFlow
// webhook actions (create_flow, run_task, …) against a bound session. The
// application id, sink id, and server URL all come from the AutoConfig token.
const pollerRouteConfigSchema = z
  .object({
    enabled: z.boolean().optional().default(true),
    // Svix AutoConfig token (`auto_v1_…`) for this route's TaskFlow actions.
    token: secretInputSchema,
    // TaskFlow session this route's actions are applied to.
    sessionKey: z.string().trim().min(1),
    controllerId: z.string().trim().min(1).optional(),
    ...sinkSubscribeFields,
    ...pollTuningFields,
    description: z.string().trim().min(1).optional(),
  })
  .strict();

const pollerPluginConfigSchema = z
  .object({
    routes: z.record(z.string().trim().min(1), pollerRouteConfigSchema).default({}),
    // --- Gateway hook pollers (optional, one of each) ---
    // Messages from `wake` are POSTed to the gateway `/hooks/wake` endpoint;
    // messages from `agent` are POSTed to `/hooks/agent`. Auth + base URL for
    // those POSTs come from the live OpenClaw `hooks`/`gateway` config.
    wake: hookEndpointSchema.optional(),
    agent: hookEndpointSchema.optional(),
  })
  .strict();

// Fields shared by every resolved poller regardless of where its messages go.
type ResolvedPollEndpoint = {
  routeId: string;
  // Human label for logs, e.g. `ops` (TaskFlow), `wake`, `agent`.
  label: string;
  token: WebhookSecretInput;
  tokenConfigPath: string;
  consumerId: string;
  subscribe: boolean;
  filterTypes?: string[];
  channels?: string[];
  startingPosition?: "earliest" | "latest";
  leaseDurationMs?: number;
  pollIntervalMs: number;
  limit: number;
  payloadField: string;
};

export type ResolvedTaskFlowPoller = ResolvedPollEndpoint & {
  kind: "taskflow";
  sessionKey: string;
  controllerId: string;
};

export type ResolvedHookPoller = ResolvedPollEndpoint & {
  // Which gateway hook this poller's messages are POSTed to.
  kind: "wake" | "agent";
};

export type ResolvedPoller = ResolvedTaskFlowPoller | ResolvedHookPoller;

// Sink-provisioning + poll-tuning resolved from either the route or a hook
// endpoint sub-object (`wake`/`agent`).
type PollTuning = {
  subscribe: boolean;
  filterTypes?: string[];
  channels?: string[];
  consumerId?: string;
  startingPosition?: "earliest" | "latest";
  leaseDurationMs?: number;
  pollIntervalMs: number;
  limit: number;
  payloadField: string;
};

function resolveEndpoint(args: {
  routeId: string;
  label: string;
  token: WebhookSecretInput;
  tokenConfigPath: string;
  tuning: PollTuning;
}): ResolvedPollEndpoint {
  const { tuning } = args;
  return {
    routeId: args.routeId,
    label: args.label,
    token: args.token,
    tokenConfigPath: args.tokenConfigPath,
    // Svix consumer group names allow only alphanumerics, '_', '-', and '.', so
    // the default separator is '.', not '/'.
    consumerId: tuning.consumerId ?? `svix-openclaw.${args.routeId}`,
    subscribe: tuning.subscribe,
    ...(tuning.filterTypes ? { filterTypes: tuning.filterTypes } : {}),
    ...(tuning.channels ? { channels: tuning.channels } : {}),
    ...(tuning.startingPosition ? { startingPosition: tuning.startingPosition } : {}),
    ...(tuning.leaseDurationMs ? { leaseDurationMs: tuning.leaseDurationMs } : {}),
    pollIntervalMs: tuning.pollIntervalMs,
    limit: tuning.limit,
    payloadField: tuning.payloadField,
  };
}

export function resolveWebhookPollerConfig(params: {
  pluginConfig: unknown;
}): ResolvedPoller[] {
  const parsed = pollerPluginConfigSchema.parse(params.pluginConfig ?? {});
  const resolved: ResolvedPoller[] = [];
  const base = `plugins.entries.svix-openclaw.config`;

  // TaskFlow pollers, one per route.
  for (const [routeId, route] of Object.entries(parsed.routes)) {
    if (!route.enabled) {
      continue;
    }
    const endpoint = resolveEndpoint({
      routeId,
      label: routeId,
      token: route.token,
      tokenConfigPath: `${base}.routes.${routeId}.token`,
      tuning: route,
    });
    resolved.push({
      ...endpoint,
      kind: "taskflow",
      sessionKey: route.sessionKey,
      controllerId: route.controllerId ?? `svix-openclaw/${routeId}`,
    });
  }

  // Gateway hook pollers (wake / agent), configured once at the top level.
  for (const kind of ["wake", "agent"] as const) {
    const hook = parsed[kind];
    if (!hook) {
      continue;
    }
    const endpoint = resolveEndpoint({
      routeId: kind,
      label: kind,
      token: hook.token,
      tokenConfigPath: `${base}.${kind}.token`,
      tuning: hook,
    });
    resolved.push({ ...endpoint, kind });
  }

  return resolved;
}
