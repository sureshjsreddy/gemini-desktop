import type {
  CliEngine,
  CliEnginesStatus,
  CliTestResult,
  GCloudStatus,
  GeminiAuthMode,
  GeminiEnvStatus,
  UserEnvSyncRequest,
  UserEnvVarStatus,
} from "$lib/types";

export interface AuthDraftSnapshot {
  authMode: GeminiAuthMode;
  apiKey: string;
  googleCloudProject: string;
  googleCloudLocation: string;
  googleAppCredentials: string;
  agyAdcAuth: boolean;
}

export function normalizeAuthDraft(draft: Partial<AuthDraftSnapshot>): AuthDraftSnapshot {
  return {
    authMode: draft.authMode === "api_key" ? "api_key" : "vertex_ai",
    apiKey: (draft.apiKey ?? "").trim(),
    googleCloudProject: (draft.googleCloudProject ?? "").trim(),
    googleCloudLocation: (draft.googleCloudLocation ?? "us-central1").trim() || "us-central1",
    googleAppCredentials: (draft.googleAppCredentials ?? "").trim(),
    agyAdcAuth: Boolean(draft.agyAdcAuth),
  };
}

export function isAuthDraftDirty(current: AuthDraftSnapshot, baseline: AuthDraftSnapshot): boolean {
  const a = normalizeAuthDraft(current);
  const b = normalizeAuthDraft(baseline);
  return (
    a.authMode !== b.authMode ||
    a.apiKey !== b.apiKey ||
    a.googleCloudProject !== b.googleCloudProject ||
    a.googleCloudLocation !== b.googleCloudLocation ||
    a.googleAppCredentials !== b.googleAppCredentials ||
    a.agyAdcAuth !== b.agyAdcAuth
  );
}

export type UserEnvSyncRowState =
  | "synced"
  | "differs"
  | "unset_in_windows"
  | "only_in_windows"
  | "unset";

export interface UserEnvComparisonRow {
  name: "GOOGLE_CLOUD_PROJECT" | "GOOGLE_CLOUD_LOCATION" | "GOOGLE_GENAI_USE_VERTEXAI" | "AGY_ADC_AUTH";
  desiredValue: string | null;
  windowsValue: string | null;
  state: UserEnvSyncRowState;
}

export interface UserEnvPlanSummary {
  rows: UserEnvComparisonRow[];
  hasPendingSync: boolean;
  hasAnyWindowsVarSet: boolean;
  detectedUserProject: string | null;
  syncRequest: UserEnvSyncRequest;
}

/**
 * Compares the modal's active GCP/Vertex configuration against persisted Windows User Environment variables.
 * Strictly manages non-secret configuration variables (no raw API keys).
 */
export function buildUserEnvComparison(
  params: {
    activeEngine: CliEngine;
    authMode: GeminiAuthMode;
    googleCloudProject: string;
    googleCloudLocation: string;
    agyAdcAuth: boolean;
    includeUseVertexFlag?: boolean;
  },
  windowsVars: UserEnvVarStatus[],
): UserEnvPlanSummary {
  const winMap = new Map<string, string>();
  for (const item of windowsVars) {
    const val = item.value?.trim();
    if (item.isSet && val) {
      winMap.set(item.name, val);
    }
  }

  const cleanProj = params.googleCloudProject.trim() || null;
  const cleanLoc = cleanProj || params.authMode === "vertex_ai"
    ? params.googleCloudLocation.trim() || null
    : null;
  const wantVertexFlag =
    params.includeUseVertexFlag ?? (params.authMode === "vertex_ai" && Boolean(cleanProj));
  const wantAgyAdc = params.activeEngine === "agy" && params.agyAdcAuth;

  const desiredEntries: Array<{
    name: UserEnvComparisonRow["name"];
    desiredValue: string | null;
  }> = [
    { name: "GOOGLE_CLOUD_PROJECT", desiredValue: cleanProj },
    { name: "GOOGLE_CLOUD_LOCATION", desiredValue: cleanLoc },
    {
      name: "GOOGLE_GENAI_USE_VERTEXAI",
      desiredValue: wantVertexFlag ? "true" : null,
    },
  ];

  if (params.activeEngine === "agy" || winMap.has("AGY_ADC_AUTH")) {
    desiredEntries.push({
      name: "AGY_ADC_AUTH",
      desiredValue: wantAgyAdc ? "true" : null,
    });
  }

  const rows: UserEnvComparisonRow[] = desiredEntries.map(({ name, desiredValue }) => {
    const windowsValue = winMap.get(name) ?? null;
    let state: UserEnvSyncRowState = "unset";
    if (desiredValue && windowsValue) {
      state = desiredValue === windowsValue ? "synced" : "differs";
    } else if (desiredValue && !windowsValue) {
      state = "unset_in_windows";
    } else if (!desiredValue && windowsValue) {
      state = "only_in_windows";
    }
    return { name, desiredValue, windowsValue, state };
  });

  const projectOrLocPending = rows.some(
    (r) =>
      (r.name === "GOOGLE_CLOUD_PROJECT" || r.name === "GOOGLE_CLOUD_LOCATION" || r.name === "GOOGLE_GENAI_USE_VERTEXAI" || r.name === "AGY_ADC_AUTH") &&
      (r.state === "unset_in_windows" || r.state === "differs"),
  );

  const syncRequest: UserEnvSyncRequest = {
    googleCloudProject: cleanProj ?? undefined,
    googleCloudLocation: cleanLoc ?? undefined,
    googleGenaiUseVertexai: wantVertexFlag ? true : undefined,
    agyAdcAuth:
      params.activeEngine === "agy"
        ? (wantAgyAdc ? true : winMap.has("AGY_ADC_AUTH") ? false : undefined)
        : undefined,
    clearUnspecified: false,
  };

  return {
    rows,
    hasPendingSync: projectOrLocPending && Boolean(cleanProj),
    hasAnyWindowsVarSet: winMap.size > 0,
    detectedUserProject: winMap.get("GOOGLE_CLOUD_PROJECT") ?? null,
    syncRequest,
  };
}

export interface DiagnosticCheckItem {
  status: "ok" | "warn" | "error" | "pending";
  label: string;
  detail: string;
}

export interface ConnectionDiagnosticsView {
  binary: DiagnosticCheckItem;
  auth: DiagnosticCheckItem;
  latency: DiagnosticCheckItem;
  model: DiagnosticCheckItem;
}

export function evaluateConnectionDiagnostics(input: {
  activeEngine: CliEngine;
  effectiveEnvStatus: GeminiEnvStatus | null;
  enginesStatus: CliEnginesStatus | null;
  authMode: GeminiAuthMode;
  apiKey: string;
  googleCloudProject: string;
  googleAppCredentials: string;
  agyAdcAuth: boolean;
  gcloudStatus: GCloudStatus | null;
  testModel: string;
  testResult: CliTestResult | null;
}): ConnectionDiagnosticsView {
  const isAgy = input.activeEngine === "agy";
  const isInstalled = isAgy
    ? Boolean(input.enginesStatus?.agyInstalled || input.effectiveEnvStatus?.installed)
    : Boolean(input.effectiveEnvStatus?.installed);
  const ver = input.effectiveEnvStatus?.version;

  const binary: DiagnosticCheckItem = isInstalled
    ? {
        status: "ok",
        label: `1. CLI Binary (${isAgy ? "agy" : "gemini"})`,
        detail: ver ? `Found (${ver.startsWith("v") ? ver : `v${ver}`})` : `Found (${isAgy ? "agy" : "gemini"} ready)`,
      }
    : {
        status: "error",
        label: `1. CLI Binary (${isAgy ? "agy" : "gemini"})`,
        detail: "Not detected on system",
      };

  const cleanProj = input.googleCloudProject.trim();
  const cleanKey = input.apiKey.trim();
  const cleanCreds = input.googleAppCredentials.trim();

  let auth: DiagnosticCheckItem;
  if (isAgy) {
    if (input.agyAdcAuth) {
      auth = cleanProj
        ? {
            status: "ok",
            label: "2. Auth Protocol",
            detail: `Enterprise ADC (${cleanProj})`,
          }
        : {
            status: "warn",
            label: "2. Auth Protocol",
            detail: "Enterprise ADC (Set GCP Project)",
          };
    } else if (input.authMode === "api_key" && cleanKey) {
      auth = {
        status: "ok",
        label: "2. Auth Protocol",
        detail: "BYOK Gemini API Key (Configured)",
      };
    } else if (cleanProj) {
      auth = {
        status: "ok",
        label: "2. Auth Protocol",
        detail: `OAuth / Vertex (${cleanProj})`,
      };
    } else if (input.testResult?.success) {
      auth = {
        status: "ok",
        label: "2. Auth Protocol",
        detail: "Antigravity Account (Verified)",
      };
    } else {
      auth = {
        status: "pending",
        label: "2. Auth Protocol",
        detail: "Antigravity OAuth (Keyring)",
      };
    }
  } else if (input.authMode === "api_key") {
    auth = cleanKey
      ? {
          status: "ok",
          label: "2. Auth Protocol",
          detail: "Gemini API Key (Configured)",
        }
      : {
          status: "warn",
          label: "2. Auth Protocol",
          detail: "Gemini API Key (Not set)",
        };
  } else {
    const hasOAuthOrAdc = Boolean(
      input.gcloudStatus?.has_gemini_oauth || input.gcloudStatus?.has_adc || cleanCreds,
    );
    if (cleanProj && hasOAuthOrAdc) {
      auth = {
        status: "ok",
        label: "2. Auth Protocol",
        detail: `Vertex AI (${cleanProj})`,
      };
    } else if (cleanProj) {
      auth = {
        status: "ok",
        label: "2. Auth Protocol",
        detail: `Vertex AI (${cleanProj})`,
      };
    } else {
      auth = {
        status: "warn",
        label: "2. Auth Protocol",
        detail: "Vertex AI (GCP Project unset)",
      };
    }
  }

  const latency: DiagnosticCheckItem = input.testResult
    ? {
        status: input.testResult.success ? "ok" : "error",
        label: "3. Latency & Ping",
        detail: `${input.testResult.latency_ms} ms`,
      }
    : {
        status: "pending",
        label: "3. Latency & Ping",
        detail: "Not tested yet",
      };

  const testedModelLabel = isAgy ? "auto (agy)" : input.testModel || "gemini-3.8-flash";
  const model: DiagnosticCheckItem = input.testResult
    ? {
        status: input.testResult.success ? "ok" : "error",
        label: "4. Model Verification",
        detail: input.testResult.success ? `${testedModelLabel} (Operational)` : "Check failed",
      }
    : {
        status: "pending",
        label: "4. Model Verification",
        detail: `${testedModelLabel} (Ready)`,
      };

  return { binary, auth, latency, model };
}
