/**
 * The one way a page talks to the platform API.
 *
 * Six pages carried their own copy and each had learned something the others
 * had not: one redirected to the login screen on 401, one knew not to set a
 * content type for a file upload, one read the error out of the body. A page
 * got whichever behaviours its copy happened to have. This is the union.
 */

/** Thrown so a caller can tell an API refusal from a network failure. */
export class ApiError extends Error {
  status: number;
  code: string;
  payload: any;

  constructor(message: string, status: number, code: string, payload: any) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.payload = payload;
  }
}

/**
 * Calls the API and answers its parsed body.
 *
 * - A 401 sends the reader to the login screen and resolves `null`, because
 *   every page's answer to an expired session is the same.
 * - A `FormData` body keeps the browser's own content type, boundary included.
 * - A failure carries the server's message rather than the status line, since
 *   the platform names the column or the constraint that refused.
 */
export async function requestJson(url: string, options: any = {}): Promise<any> {
  const isFormData = typeof FormData !== "undefined" && options.body instanceof FormData;
  const response = await fetch(url, {
    ...options,
    headers: {
      Accept: "application/json",
      ...(options.body && !isFormData ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
  });

  if (response.status === 401) {
    if (typeof window !== "undefined") window.location.href = "/login";
    return null;
  }

  const payload = await response.json().catch(() => null);
  if (!response.ok) {
    const message =
      payload?.error?.message ||
      payload?.error ||
      payload?.message ||
      `${response.status} ${response.statusText}`;
    throw new ApiError(
      String(message),
      response.status,
      String(payload?.error?.code || ""),
      payload,
    );
  }
  return payload;
}
