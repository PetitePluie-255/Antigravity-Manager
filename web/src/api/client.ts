const API_BASE = import.meta.env.VITE_API_URL || "/api";

export async function request<T>(
  path: string,
  options?: RequestInit
): Promise<T> {
  const url = `${API_BASE}${path}`;
  const headers = {
    "Content-Type": "application/json",
    ...options?.headers,
  };

  const response = await fetch(url, { ...options, headers });

  if (response.status === 204) {
    return {} as T;
  }

  const json = await response.json();

  if (!json.success && json.error) {
    throw new Error(json.error);
  }

  if (json.success) {
    return json.data as T;
  }

  // Fallback if not wrapped
  return json as T;
}
