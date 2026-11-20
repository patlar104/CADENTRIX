export async function apiRequest(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    cache: "no-store",
    headers: {
      ...options.headers,
      ...(options.body ? { "Content-Type": "application/json" } : {}),
    },
  });
  let data;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      "The backend did not return a valid response. Check that the local service is running.",
    );
  }
  if (!response.ok) {
    throw new Error(
      data.error?.message || "The backend could not complete this request.",
    );
  }
  return data;
}
