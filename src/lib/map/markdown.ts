// A *very* small markdown-to-HTML converter for marker descriptions.
//
// !!! Cybersecurity studies note (important) !!!
// The string this function returns is later inserted into the page with
// `innerHTML`. Inserting HTML you built from text is exactly the pattern that
// leads to Cross-Site Scripting (XSS) — if the input text contained something
// like `<img onerror=...>`, it could run. In a real security review we would
// sanitise the input or build DOM nodes with textContent instead.
// Here the description text comes from MapGenie's own dataset (which we already
// downloaded and treat as semi-trusted), and we only translate a tiny, fixed set
// of markers (**bold**, *italic*, newlines), so the risk stays low — but it is
// worth flagging loudly so a future reader knows to be careful.

export function simpleMarkdownToHtml(text: string): string {
  return text
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/\n/g, '<br>');
}
