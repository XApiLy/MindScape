/** Keeps Markdown line breaks intact while simulating small streamed deltas. */
export function chunkMockResponse(response: string) {
  return response.match(/.{1,9}/gsu) ?? [response];
}
