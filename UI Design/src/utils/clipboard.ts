/**
 * Safely copies text to clipboard.
 *
 * Inside Figma Make's sandboxed iframe the async Clipboard API is blocked by a
 * permissions policy and rejects with NotAllowedError. To avoid surfacing that
 * rejection to the console on every copy, we use the legacy execCommand path
 * first (it works within the iframe given a user gesture) and only fall back to
 * the async Clipboard API when execCommand is unavailable.
 */
function legacyCopy(text: string): boolean {
  try {
    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.setAttribute('readonly', '')
    textarea.style.position = 'fixed'
    textarea.style.left = '-9999px'
    textarea.style.top = '-9999px'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)
    textarea.focus()
    textarea.select()
    const successful = document.execCommand('copy')
    document.body.removeChild(textarea)
    return successful
  } catch {
    return false
  }
}

export async function safeCopyText(text: string): Promise<boolean> {
  if (!text) return false

  // Prefer the legacy path first — avoids the blocked Clipboard API in iframes.
  if (legacyCopy(text)) return true

  // Fallback: modern Clipboard API (used in non-sandboxed contexts).
  if (navigator?.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      /* silently ignore — copy is best-effort */
    }
  }

  return false
}
