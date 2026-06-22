import { definePluginEntry, type OpenClawPluginApi } from "./api.js";
import type { OpenClawConfig } from "./runtime-api.js";
import {
  resolveWebhookPollerConfig,
  type ResolvedHookPoller,
  type ResolvedTaskFlowPoller,
} from "./src/config.js";
import { createWebhookPoller, type DispatchFn, type WebhookPoller } from "./src/poller.js";
import { describeActionForLog, processWebhookAction } from "./src/processor.js";
import type { TaskFlowWebhookTarget } from "./src/vendor/webhook-actions.js";

// Resolve the local gateway hooks base URL + bearer token from the live config.
// The gateway binds loopback by default, so hook POSTs go to 127.0.0.1.
function resolveHooksTarget(cfg: OpenClawConfig): {
  baseUrl: string;
  token?: string;
  enabled: boolean;
} {
  const port = cfg.gateway?.port ?? 18789;
  const path = (cfg.hooks?.path ?? "/hooks").replace(/\/+$/, "");
  return {
    baseUrl: `http://127.0.0.1:${port}${path}`,
    token: cfg.hooks?.token,
    enabled: cfg.hooks?.enabled === true,
  };
}

// TaskFlow poller: apply each polled payload as an upstream webhook action.
function makeTaskFlowDispatch(api: OpenClawPluginApi, poller: ResolvedTaskFlowPoller): DispatchFn {
  const taskFlow = api.runtime.tasks.managedFlows.bindSession({ sessionKey: poller.sessionKey });
  // Reuse the upstream target shape so the vendored action executor is
  // unchanged. `secretInput`/`secretConfigPath` carry the polling token here
  // (the polling transport is the client, so there is no inbound secret to
  // verify); the executor only reads `taskFlow` and `defaultControllerId`.
  const target: TaskFlowWebhookTarget = {
    routeId: poller.routeId,
    path: `svix:${poller.consumerId}`,
    secretInput: poller.token,
    secretConfigPath: poller.tokenConfigPath,
    defaultControllerId: poller.controllerId,
    taskFlow,
  };
  return async (action) => {
    const outcome = await processWebhookAction({ action, target, cfg: api.config });
    return {
      ok: outcome.ok,
      status: outcome.statusCode,
      ...(outcome.code ? { code: outcome.code } : {}),
      ...(outcome.error ? { error: outcome.error } : {}),
      summary: describeActionForLog(action),
    };
  };
}

// Hook poller: POST each polled payload to the gateway /hooks/wake or
// /hooks/agent endpoint, using the documented bearer-token auth.
function makeHookDispatch(api: OpenClawPluginApi, poller: ResolvedHookPoller): DispatchFn {
  return async (action) => {
    const hooks = resolveHooksTarget(api.config);
    if (!hooks.enabled) {
      return { ok: false, code: "hooks_disabled", error: "set hooks.enabled=true in the OpenClaw config" };
    }
    if (!hooks.token) {
      return { ok: false, code: "hooks_token_missing", error: "set hooks.token in the OpenClaw config" };
    }
    const url = `${hooks.baseUrl}/${poller.kind}`;
    let res: Response;
    try {
      res = await fetch(url, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${hooks.token}`,
        },
        body: JSON.stringify(action ?? {}),
      });
    } catch (err) {
      return { ok: false, code: "network_error", error: String(err) };
    }
    if (!res.ok) {
      const detail = await res.text().catch(() => "");
      return { ok: false, status: res.status, code: `http_${res.status}`, error: detail.slice(0, 300) };
    }
    return { ok: true, status: res.status, summary: poller.kind };
  };
}

function registerWebhookPollers(api: OpenClawPluginApi): void {
  const resolved = resolveWebhookPollerConfig({ pluginConfig: api.pluginConfig });
  if (resolved.length === 0) {
    return;
  }

  const pollers: WebhookPoller[] = [];
  for (const poller of resolved) {
    const dispatch =
      poller.kind === "taskflow"
        ? makeTaskFlowDispatch(api, poller)
        : makeHookDispatch(api, poller);
    pollers.push(createWebhookPoller({ poller, cfg: api.config, logger: api.logger, dispatch }));
  }

  // Drive the pollers through the plugin service lifecycle so they start after
  // the runtime is live and stop cleanly on shutdown/reload.
  api.registerService({
    id: "svix-openclaw",
    start() {
      for (const poller of pollers) {
        poller.start();
      }
      api.logger.info?.(`[svix-openclaw] started ${pollers.length} poller(s)`);
    },
    async stop() {
      await Promise.all(pollers.map((poller) => poller.stop()));
    },
  });
}

export default definePluginEntry({
  id: "svix-openclaw",
  name: "Svix OpenClaw",
  description:
    "Receives OpenClaw webhook actions by polling Svix Polling Endpoints instead of exposing an inbound HTTP server: applies TaskFlow actions and/or forwards messages to the gateway /hooks/wake and /hooks/agent automation endpoints.",
  register(api: OpenClawPluginApi) {
    registerWebhookPollers(api);
  },
});
