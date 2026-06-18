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

// Poll tuning shared by every Svix Polling Endpoint (the TaskFlow URL and the
// per-hook URLs). Spread into each endpoint schema so each poller is tuned
// independently.
const pollTuningFields = {
  // Optional Svix-side filters applied by the polling endpoint.
  eventType: z.string().trim().min(1).optional(),
  channel: z.string().trim().min(1).optional(),
  // Idle wait (ms) after the endpoint reports it is caught up (`done: true`).
  pollIntervalMs: z.number().int().positive().optional().default(5_000),
  // Page size requested per poll.
  limit: z.number().int().positive().max(500).optional().default(50),
  // Resume cursor to seed the first poll (otherwise starts from the tail).
  startIterator: z.string().trim().min(1).optional(),
  // Field on each polled message that holds the action/body object. Defaults to
  // the Svix message `payload`. Empty string means the whole message object.
  payloadField: z.string().optional().default("payload"),
};

// A Svix Polling Endpoint whose buffered messages are forwarded to one of the
// OpenClaw gateway hooks (`/hooks/wake` or `/hooks/agent`). Each message's
// payload is used verbatim as the hook request body.
const hookEndpointSchema = z
  .object({
    // Svix Polling Endpoint URL, e.g.
    // https://api.svix.com/api/v1/app/app_xxx/poller/poll_xxx
    url: z.string().trim().url(),
    // Svix auth token. Inline string or a `{ source, provider, id }` secret ref.
    token: secretInputSchema,
    ...pollTuningFields,
  })
  .strict();

// A TaskFlow route: a Svix Polling Endpoint whose messages are applied as
// TaskFlow webhook actions (create_flow, run_task, …) against a bound session.
// The server URL, application id, and sink id are parsed from `url`.
const pollerRouteConfigSchema = z
  .object({
    enabled: z.boolean().optional().default(true),
    // Svix Polling Endpoint URL for this route's TaskFlow actions.
    url: z.string().trim().url(),
    // Svix auth token for the TaskFlow `url`.
    token: secretInputSchema,
    // TaskFlow session this route's actions are applied to.
    sessionKey: z.string().trim().min(1),
    controllerId: z.string().trim().min(1).optional(),
    ...pollTuningFields,
    description: z.string().trim().min(1).optional(),
  })
  .strict();

const pollerPluginConfigSchema = z
  .object({
    routes: z.record(z.string().trim().min(1), pollerRouteConfigSchema).default({}),
    // --- Gateway hook pollers (optional, one of each) ---
    // Messages from `wake.url` are POSTed to the gateway `/hooks/wake` endpoint;
    // messages from `agent.url` are POSTed to `/hooks/agent`. Auth + base URL
    // for those POSTs come from the live OpenClaw `hooks`/`gateway` config.
    wake: hookEndpointSchema.optional(),
    agent: hookEndpointSchema.optional(),
  })
  .strict();

// Fields shared by every resolved poller regardless of where its messages go.
type ResolvedPollEndpoint = {
  routeId: string;
  // Human label for logs, e.g. `ops` (TaskFlow), `wake`, `agent`.
  label: string;
  appId: string;
  sinkId: string;
  serverUrl: string;
  token: WebhookSecretInput;
  tokenConfigPath: string;
  eventType?: string;
  channel?: string;
  pollIntervalMs: number;
  limit: number;
  startIterator?: string;
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

// A Svix Polling Endpoint URL embeds the server origin, application id, and sink
// id, e.g. https://api.svix.com/api/v1/app/<appId>/poller/<sinkId>. The SDK
// `message.poller.poll(appId, sinkId)` call takes those parts separately and the
// origin as the client `serverUrl`, so we split the URL back into them here.
function parsePollEndpointUrl(rawUrl: string): {
  serverUrl: string;
  appId: string;
  sinkId: string;
} {
  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch {
    throw new Error(`svix-openclaw: invalid Svix poll endpoint url: ${JSON.stringify(rawUrl)}`);
  }

  const segments = parsed.pathname.split("/").filter((segment) => segment.length > 0);
  const appIdx = segments.indexOf("app");
  const pollerIdx = segments.indexOf("poller");
  const appId = appIdx >= 0 ? segments[appIdx + 1] : undefined;
  const sinkId = pollerIdx >= 0 ? segments[pollerIdx + 1] : undefined;

  if (!appId || !sinkId) {
    throw new Error(
      `svix-openclaw: could not parse application id and sink id from poll endpoint url ` +
        `${JSON.stringify(rawUrl)}; expected a path like /api/v1/app/<appId>/poller/<sinkId>`,
    );
  }

  return { serverUrl: parsed.origin, appId, sinkId };
}

// Endpoint-level tuning resolved from either the route (TaskFlow `url`) or a
// hook endpoint sub-object (`wake`/`agent`).
type PollTuning = {
  eventType?: string;
  channel?: string;
  pollIntervalMs: number;
  limit: number;
  startIterator?: string;
  payloadField: string;
};

function resolveEndpoint(args: {
  routeId: string;
  label: string;
  url: string;
  token: WebhookSecretInput;
  tokenConfigPath: string;
  tuning: PollTuning;
}): ResolvedPollEndpoint {
  const { serverUrl, appId, sinkId } = parsePollEndpointUrl(args.url);
  return {
    routeId: args.routeId,
    label: args.label,
    appId,
    sinkId,
    serverUrl,
    token: args.token,
    tokenConfigPath: args.tokenConfigPath,
    ...(args.tuning.eventType ? { eventType: args.tuning.eventType } : {}),
    ...(args.tuning.channel ? { channel: args.tuning.channel } : {}),
    pollIntervalMs: args.tuning.pollIntervalMs,
    limit: args.tuning.limit,
    ...(args.tuning.startIterator ? { startIterator: args.tuning.startIterator } : {}),
    payloadField: args.tuning.payloadField,
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
      url: route.url,
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
      url: hook.url,
      token: hook.token,
      tokenConfigPath: `${base}.${kind}.token`,
      tuning: hook,
    });
    resolved.push({ ...endpoint, kind });
  }

  return resolved;
}
