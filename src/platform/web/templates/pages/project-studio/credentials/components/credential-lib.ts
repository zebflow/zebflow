// Small shared helpers of the credentials page.

export const FALLBACK_KINDS = [
  "postgres", "mysql", "openai", "openrouter", "http", "github", "gitlab",
  "jwt_signing_key", "browser_browserless", "oauth2", "hmac", "api_key", "tts", "s3", "secure_request", "custom",
];

export const REQUEST_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];

export function listToText(value: any): string {
  return Array.isArray(value) ? value.map((item) => String(item ?? "")).filter(Boolean).join("\n") : "";
}

export function textToList(value: string): string[] {
  return String(value || "").split(/[\n,]+/).map((item) => item.trim()).filter(Boolean);
}

export function defaultSecretForKind(kind: string, types: any[]): Record<string, any> {
  // Complex types with interactive defaults
  if (kind === "oauth2") {
    return {
      provider: "", client_id: "", client_secret: "", authorize_url: "",
      token_url: "", scopes: "", redirect_uri: "", refresh_token: "",
      access_token: "", expires_at: 0, token_type: "Bearer",
    };
  }
  if (kind === "hmac") {
    return { secrets: { current: "" }, signed: "{body}", signature: { from: "header:X-Signature", encoding: "hex" } };
  }
  // Derive defaults from API field definitions
  const typeDef = types.find((t) => t.kind === kind);
  if (!typeDef) return {};
  const defaults: Record<string, any> = {};
  for (const field of typeDef.fields || []) {
    if (field.default != null) defaults[field.key] = field.default;
  }
  return defaults;
}

export function generateHex(bytes: number): string {
  const arr = new Uint8Array(bytes);
  crypto.getRandomValues(arr);
  return Array.from(arr).map((b) => b.toString(16).padStart(2, "0")).join("");
}
