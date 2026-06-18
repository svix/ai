/**
 * VENDORED FROM OPENCLAW — keep in sync, do not hand-edit the logic.
 *
 * Source: openclaw `extensions/webhooks/src/http.ts`
 * (the OpenClaw webhook bridge that binds inbound automation to TaskFlows).
 *
 * This file is the transport-agnostic SUBSET of the upstream module: every
 * request schema, view mapper, status mapper, plus `executeWebhookAction` and
 * `describeWebhookOutcome` are copied VERBATIM so they can be re-synced with a
 * near-mechanical diff against upstream.
 *
 * Intentionally OMITTED from upstream (HTTP-transport only — replaced by the
 * polling transport in `../poller.ts`):
 *   - `writeJson`, `extractSharedSecret`, `timingSafeEquals`
 *   - `createTaskFlowWebhookRequestHandler` (the Node http handler + rate
 *     limiting / in-flight limiting / body reading / shared-secret auth)
 *
 * The ONLY deltas from upstream are:
 *   1. The three import lines below repoint at this plugin's SDK shims.
 *   2. `export` is added to the symbols the polling transport consumes
 *      (`webhookActionSchema`, `WebhookAction`, `TaskFlowWebhookTarget`,
 *      `formatZodError`, `executeWebhookAction`, `describeWebhookOutcome`).
 *
 * To update: copy upstream `http.ts`, drop the omitted HTTP functions, and
 * re-apply deltas (1) and (2).
 */
import { z } from "zod";
import type { PluginRuntime } from "../../api.js";
import type { OpenClawConfig } from "../../runtime-api.js";
import type { WebhookSecretInput } from "../config.js";

type BoundTaskFlowRuntime = ReturnType<PluginRuntime["tasks"]["managedFlows"]["bindSession"]>;

type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

const jsonValueSchema: z.ZodType<JsonValue> = z.lazy(() =>
  z.union([
    z.null(),
    z.boolean(),
    z.number().finite(),
    z.string(),
    z.array(jsonValueSchema),
    z.record(z.string(), jsonValueSchema),
  ]),
);

const nullableStringSchema = z.string().trim().min(1).nullable().optional();

const createFlowRequestSchema = z
  .object({
    action: z.literal("create_flow"),
    controllerId: z.string().trim().min(1).optional(),
    goal: z.string().trim().min(1),
    status: z.enum(["queued", "running", "waiting", "blocked"]).optional(),
    notifyPolicy: z.enum(["done_only", "state_changes", "silent"]).optional(),
    currentStep: nullableStringSchema,
    stateJson: jsonValueSchema.nullable().optional(),
    waitJson: jsonValueSchema.nullable().optional(),
  })
  .strict();

const getFlowRequestSchema = z
  .object({ action: z.literal("get_flow"), flowId: z.string().trim().min(1) })
  .strict();
const listFlowsRequestSchema = z.object({ action: z.literal("list_flows") }).strict();
const findLatestFlowRequestSchema = z.object({ action: z.literal("find_latest_flow") }).strict();
const resolveFlowRequestSchema = z
  .object({ action: z.literal("resolve_flow"), token: z.string().trim().min(1) })
  .strict();
const getTaskSummaryRequestSchema = z
  .object({ action: z.literal("get_task_summary"), flowId: z.string().trim().min(1) })
  .strict();

const setWaitingRequestSchema = z
  .object({
    action: z.literal("set_waiting"),
    flowId: z.string().trim().min(1),
    expectedRevision: z.number().int().nonnegative(),
    currentStep: nullableStringSchema,
    stateJson: jsonValueSchema.nullable().optional(),
    waitJson: jsonValueSchema.nullable().optional(),
    blockedTaskId: nullableStringSchema,
    blockedSummary: nullableStringSchema,
  })
  .strict();

const resumeFlowRequestSchema = z
  .object({
    action: z.literal("resume_flow"),
    flowId: z.string().trim().min(1),
    expectedRevision: z.number().int().nonnegative(),
    status: z.enum(["queued", "running"]).optional(),
    currentStep: nullableStringSchema,
    stateJson: jsonValueSchema.nullable().optional(),
  })
  .strict();

const finishFlowRequestSchema = z
  .object({
    action: z.literal("finish_flow"),
    flowId: z.string().trim().min(1),
    expectedRevision: z.number().int().nonnegative(),
    stateJson: jsonValueSchema.nullable().optional(),
  })
  .strict();

const failFlowRequestSchema = z
  .object({
    action: z.literal("fail_flow"),
    flowId: z.string().trim().min(1),
    expectedRevision: z.number().int().nonnegative(),
    stateJson: jsonValueSchema.nullable().optional(),
    blockedTaskId: nullableStringSchema,
    blockedSummary: nullableStringSchema,
  })
  .strict();

const requestCancelRequestSchema = z
  .object({
    action: z.literal("request_cancel"),
    flowId: z.string().trim().min(1),
    expectedRevision: z.number().int().nonnegative(),
  })
  .strict();

const cancelFlowRequestSchema = z
  .object({
    action: z.literal("cancel_flow"),
    flowId: z.string().trim().min(1),
  })
  .strict();

const runTaskRequestSchema = z
  .object({
    action: z.literal("run_task"),
    flowId: z.string().trim().min(1),
    runtime: z.enum(["subagent", "acp"]),
    sourceId: z.string().trim().min(1).optional(),
    childSessionKey: z.string().trim().min(1).optional(),
    parentTaskId: z.string().trim().min(1).optional(),
    agentId: z.string().trim().min(1).optional(),
    runId: z.string().trim().min(1).optional(),
    label: z.string().trim().min(1).optional(),
    task: z.string().trim().min(1),
    preferMetadata: z.boolean().optional(),
    notifyPolicy: z.enum(["done_only", "state_changes", "silent"]).optional(),
    status: z.enum(["queued", "running"]).optional(),
    startedAt: z.number().int().nonnegative().optional(),
    lastEventAt: z.number().int().nonnegative().optional(),
    progressSummary: nullableStringSchema,
  })
  .strict()
  .superRefine((value, ctx) => {
    if (
      value.status !== "running" &&
      (value.startedAt !== undefined ||
        value.lastEventAt !== undefined ||
        value.progressSummary !== undefined)
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message:
          "status must be running when startedAt, lastEventAt, or progressSummary is provided",
        path: ["status"],
      });
    }
  });

export const webhookActionSchema = z.discriminatedUnion("action", [
  createFlowRequestSchema,
  getFlowRequestSchema,
  listFlowsRequestSchema,
  findLatestFlowRequestSchema,
  resolveFlowRequestSchema,
  getTaskSummaryRequestSchema,
  setWaitingRequestSchema,
  resumeFlowRequestSchema,
  finishFlowRequestSchema,
  failFlowRequestSchema,
  requestCancelRequestSchema,
  cancelFlowRequestSchema,
  runTaskRequestSchema,
]);

export type WebhookAction = z.infer<typeof webhookActionSchema>;

export type TaskFlowWebhookTarget = {
  routeId: string;
  path: string;
  secretInput: WebhookSecretInput;
  secretConfigPath: string;
  defaultControllerId: string;
  taskFlow: BoundTaskFlowRuntime;
};

type FlowView = {
  flowId: string;
  syncMode: "task_mirrored" | "managed";
  controllerId?: string;
  revision: number;
  status: string;
  notifyPolicy: string;
  goal: string;
  currentStep?: string;
  blockedTaskId?: string;
  blockedSummary?: string;
  stateJson?: JsonValue;
  waitJson?: JsonValue;
  cancelRequestedAt?: number;
  createdAt: number;
  updatedAt: number;
  endedAt?: number;
};

type TaskView = {
  taskId: string;
  runtime: string;
  sourceId?: string;
  scopeKind: string;
  childSessionKey?: string;
  parentFlowId?: string;
  parentTaskId?: string;
  agentId?: string;
  runId?: string;
  label?: string;
  task: string;
  status: string;
  deliveryStatus: string;
  notifyPolicy: string;
  createdAt: number;
  startedAt?: number;
  endedAt?: number;
  lastEventAt?: number;
  cleanupAfter?: number;
  error?: string;
  progressSummary?: string;
  terminalSummary?: string;
  terminalOutcome?: string;
};

function pickOptionalFields<T extends object, TKey extends keyof T & string>(
  source: T,
  keys: readonly TKey[],
): Partial<Pick<T, TKey>> {
  const result: Partial<Pick<T, TKey>> = {};
  for (const key of keys) {
    const value = source[key];
    if (value !== undefined) {
      result[key] = value;
    }
  }
  return result;
}

function pickOptionalTruthyStringFields<T extends object, TKey extends keyof T & string>(
  source: T,
  keys: readonly TKey[],
): Partial<Pick<T, TKey>> {
  const result: Partial<Pick<T, TKey>> = {};
  for (const key of keys) {
    const value = source[key];
    if (typeof value === "string" && value) {
      result[key] = value as T[TKey];
    }
  }
  return result;
}

function toFlowView(flow: FlowView): FlowView {
  return {
    flowId: flow.flowId,
    syncMode: flow.syncMode,
    ...pickOptionalTruthyStringFields(flow, [
      "controllerId",
      "currentStep",
      "blockedTaskId",
      "blockedSummary",
    ]),
    revision: flow.revision,
    status: flow.status,
    notifyPolicy: flow.notifyPolicy,
    goal: flow.goal,
    ...pickOptionalFields(flow, ["stateJson", "waitJson", "cancelRequestedAt"]),
    createdAt: flow.createdAt,
    updatedAt: flow.updatedAt,
    ...pickOptionalFields(flow, ["endedAt"]),
  };
}

function toTaskView(task: TaskView): TaskView {
  return {
    taskId: task.taskId,
    runtime: task.runtime,
    ...pickOptionalTruthyStringFields(task, [
      "sourceId",
      "childSessionKey",
      "parentFlowId",
      "parentTaskId",
      "agentId",
      "runId",
      "label",
      "error",
      "progressSummary",
      "terminalSummary",
      "terminalOutcome",
    ]),
    scopeKind: task.scopeKind,
    task: task.task,
    status: task.status,
    deliveryStatus: task.deliveryStatus,
    notifyPolicy: task.notifyPolicy,
    createdAt: task.createdAt,
    ...pickOptionalFields(task, ["startedAt", "endedAt", "lastEventAt", "cleanupAfter"]),
  };
}

export function formatZodError(error: z.ZodError): string {
  const firstIssue = error.issues[0];
  if (!firstIssue) {
    return "invalid request";
  }
  const path = firstIssue.path.length > 0 ? `${firstIssue.path.join(".")}: ` : "";
  return `${path}${firstIssue.message}`;
}

function mapMutationResult(
  result:
    | {
        applied: true;
        flow: FlowView;
      }
    | {
        applied: false;
        code: string;
        current?: FlowView;
      },
): unknown {
  return result;
}

function mapFlowMutationResult(
  result:
    | {
        applied: true;
        flow: Parameters<typeof toFlowView>[0];
      }
    | {
        applied: false;
        code: string;
        current?: Parameters<typeof toFlowView>[0];
      },
): unknown {
  return mapMutationResult(
    result.applied
      ? { applied: true, flow: toFlowView(result.flow) }
      : {
          applied: false,
          code: result.code,
          ...(result.current ? { current: toFlowView(result.current) } : {}),
        },
  );
}

function mapMutationStatus(result: {
  applied: boolean;
  code?: "not_found" | "not_managed" | "revision_conflict" | "persist_failed";
}): { statusCode: number; code?: string; error?: string } {
  if (result.applied) {
    return { statusCode: 200 };
  }
  switch (result.code) {
    case "not_found":
      return {
        statusCode: 404,
        code: "not_found",
        error: "TaskFlow not found.",
      };
    case "not_managed":
      return {
        statusCode: 409,
        code: "not_managed",
        error: "TaskFlow is not managed by this webhook surface.",
      };
    case "revision_conflict":
      return {
        statusCode: 409,
        code: "revision_conflict",
        error: "TaskFlow changed since the caller's expected revision.",
      };
    case "persist_failed":
      return {
        statusCode: 503,
        code: "persist_failed",
        error: "TaskFlow persistence failed.",
      };
    default:
      return {
        statusCode: 409,
        code: "mutation_rejected",
        error: "TaskFlow mutation was rejected.",
      };
  }
}

function mapCreateFlowStatus(result: { created: boolean; code?: "persist_failed" }): {
  statusCode: number;
  code?: string;
  error?: string;
} {
  if (result.created) {
    return { statusCode: 200 };
  }
  if (result.code === "persist_failed") {
    return {
      statusCode: 503,
      code: "persist_failed",
      error: "TaskFlow persistence failed.",
    };
  }
  return {
    statusCode: 409,
    code: "create_rejected",
    error: "TaskFlow creation was rejected.",
  };
}

function mapRunTaskStatus(result: { created: boolean; found: boolean; reason?: string }): {
  statusCode: number;
  code?: string;
  error?: string;
} {
  if (result.created) {
    return { statusCode: 200 };
  }
  if (!result.found) {
    return {
      statusCode: 404,
      code: "not_found",
      error: "TaskFlow not found.",
    };
  }
  if (result.reason === "Flow cancellation has already been requested.") {
    return {
      statusCode: 409,
      code: "cancel_requested",
      error: result.reason,
    };
  }
  if (result.reason === "Flow does not accept managed child tasks.") {
    return {
      statusCode: 409,
      code: "not_managed",
      error: result.reason,
    };
  }
  if (result.reason?.startsWith("Flow is already ")) {
    return {
      statusCode: 409,
      code: "terminal",
      error: result.reason,
    };
  }
  if (result.reason === "Task persistence failed.") {
    return {
      statusCode: 503,
      code: "persist_failed",
      error: result.reason,
    };
  }
  return {
    statusCode: 409,
    code: "task_not_created",
    error: result.reason ?? "TaskFlow task was not created.",
  };
}

function mapCancelStatus(result: { found: boolean; cancelled: boolean; reason?: string }): {
  statusCode: number;
  code?: string;
  error?: string;
} {
  if (result.cancelled) {
    return { statusCode: 200 };
  }
  if (!result.found) {
    return {
      statusCode: 404,
      code: "not_found",
      error: "TaskFlow not found.",
    };
  }
  if (result.reason === "One or more child tasks are still active.") {
    return {
      statusCode: 202,
      code: "cancel_pending",
      error: result.reason,
    };
  }
  if (result.reason === "Flow changed while cancellation was in progress.") {
    return {
      statusCode: 409,
      code: "revision_conflict",
      error: result.reason,
    };
  }
  if (result.reason?.startsWith("Flow is already ")) {
    return {
      statusCode: 409,
      code: "terminal",
      error: result.reason,
    };
  }
  if (result.reason === "Flow persistence failed.") {
    return {
      statusCode: 503,
      code: "persist_failed",
      error: result.reason,
    };
  }
  return {
    statusCode: 409,
    code: "cancel_rejected",
    error: result.reason ?? "TaskFlow cancellation was rejected.",
  };
}

export function describeWebhookOutcome(params: { action: WebhookAction; result: unknown }): {
  statusCode: number;
  code?: string;
  error?: string;
} {
  switch (params.action.action) {
    case "create_flow":
      return mapCreateFlowStatus(
        params.result as {
          created: boolean;
          code?: "persist_failed";
        },
      );
    case "set_waiting":
    case "resume_flow":
    case "finish_flow":
    case "fail_flow":
    case "request_cancel":
      return mapMutationStatus(
        params.result as {
          applied: boolean;
          code?: "not_found" | "not_managed" | "revision_conflict" | "persist_failed";
        },
      );
    case "cancel_flow":
      return mapCancelStatus(
        params.result as {
          found: boolean;
          cancelled: boolean;
          reason?: string;
        },
      );
    case "run_task":
      return mapRunTaskStatus(
        params.result as {
          created: boolean;
          found: boolean;
          reason?: string;
        },
      );
    default:
      return { statusCode: 200 };
  }
}

export async function executeWebhookAction(params: {
  action: WebhookAction;
  target: TaskFlowWebhookTarget;
  cfg: OpenClawConfig;
}): Promise<unknown> {
  const { action, target } = params;
  switch (action.action) {
    case "create_flow": {
      const flow = target.taskFlow.tryCreateManaged({
        controllerId: action.controllerId ?? target.defaultControllerId,
        goal: action.goal,
        status: action.status,
        notifyPolicy: action.notifyPolicy,
        currentStep: action.currentStep ?? undefined,
        stateJson: action.stateJson,
        waitJson: action.waitJson,
      });
      return flow
        ? { created: true, flow: toFlowView(flow) }
        : { created: false, code: "persist_failed" };
    }
    case "get_flow": {
      const flow = target.taskFlow.get(action.flowId);
      return { flow: flow ? toFlowView(flow) : null };
    }
    case "list_flows":
      return { flows: target.taskFlow.list().map(toFlowView) };
    case "find_latest_flow": {
      const flow = target.taskFlow.findLatest();
      return { flow: flow ? toFlowView(flow) : null };
    }
    case "resolve_flow": {
      const flow = target.taskFlow.resolve(action.token);
      return { flow: flow ? toFlowView(flow) : null };
    }
    case "get_task_summary":
      return { summary: target.taskFlow.getTaskSummary(action.flowId) ?? null };
    case "set_waiting": {
      const result = target.taskFlow.setWaiting({
        flowId: action.flowId,
        expectedRevision: action.expectedRevision,
        currentStep: action.currentStep,
        stateJson: action.stateJson,
        waitJson: action.waitJson,
        blockedTaskId: action.blockedTaskId,
        blockedSummary: action.blockedSummary,
      });
      return mapFlowMutationResult(result);
    }
    case "resume_flow": {
      const result = target.taskFlow.resume({
        flowId: action.flowId,
        expectedRevision: action.expectedRevision,
        status: action.status,
        currentStep: action.currentStep,
        stateJson: action.stateJson,
      });
      return mapFlowMutationResult(result);
    }
    case "finish_flow": {
      const result = target.taskFlow.finish({
        flowId: action.flowId,
        expectedRevision: action.expectedRevision,
        stateJson: action.stateJson,
      });
      return mapFlowMutationResult(result);
    }
    case "fail_flow": {
      const result = target.taskFlow.fail({
        flowId: action.flowId,
        expectedRevision: action.expectedRevision,
        stateJson: action.stateJson,
        blockedTaskId: action.blockedTaskId,
        blockedSummary: action.blockedSummary,
      });
      return mapFlowMutationResult(result);
    }
    case "request_cancel": {
      const result = target.taskFlow.requestCancel({
        flowId: action.flowId,
        expectedRevision: action.expectedRevision,
      });
      return mapFlowMutationResult(result);
    }
    case "cancel_flow": {
      const result = await target.taskFlow.cancel({
        flowId: action.flowId,
        cfg: params.cfg,
      });
      return {
        found: result.found,
        cancelled: result.cancelled,
        ...(result.reason ? { reason: result.reason } : {}),
        ...(result.flow ? { flow: toFlowView(result.flow) } : {}),
        ...(result.tasks ? { tasks: result.tasks.map(toTaskView) } : {}),
      };
    }
    case "run_task": {
      const result = target.taskFlow.runTask({
        flowId: action.flowId,
        runtime: action.runtime,
        sourceId: action.sourceId,
        childSessionKey: action.childSessionKey,
        parentTaskId: action.parentTaskId,
        agentId: action.agentId,
        runId: action.runId,
        label: action.label,
        task: action.task,
        preferMetadata: action.preferMetadata,
        notifyPolicy: action.notifyPolicy,
        status: action.status,
        startedAt: action.startedAt,
        lastEventAt: action.lastEventAt,
        progressSummary: action.progressSummary,
      });
      if (result.created) {
        return {
          created: true,
          flow: toFlowView(result.flow),
          task: toTaskView(result.task),
        };
      }
      return {
        found: result.found,
        created: false,
        reason: result.reason,
        ...(result.flow ? { flow: toFlowView(result.flow) } : {}),
      };
    }
  }
  throw new Error("Unsupported webhook action");
}
